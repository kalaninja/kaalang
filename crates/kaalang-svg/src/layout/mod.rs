//! Places the visual topology on rows and columns, routes its connections, and
//! positions the labels its exits and nodes own.

use kaalang_model::topology::{Destination, ExitId, NodeId, NodeKind, Source, Topology, Vertex};
use kaalang_model::{Arrangement, SemanticModel};
use syn::{ReturnType, Signature, spanned::Spanned};
use unicode_segmentation::UnicodeSegmentation;

use crate::captions::{self, Captions};

mod action;
mod choice;
mod end;
mod label;
mod question;
mod route;
#[cfg(test)]
mod tests;
mod text;

use label::{label_rect, vertical_gap};
use text::wrap_text;

const MARGIN: i32 = 32;
const COLUMN_WIDTH: i32 = 360;
const MIN_VERTICAL_GAP: i32 = 72;
const NODE_WIDTH: i32 = 280;
/// Leaves the start hand-over label clear beneath the link to the panel.
const PARAMETER_PANEL_GAP: i32 = 96;
const PARAMETER_PANEL_WIDTH: i32 = 240;
const PARAMETER_LABEL_WIDTH: i32 = PARAMETER_PANEL_WIDTH - 32;
const CASE_WIDTH: i32 = 240;
pub(crate) const CASE_TIP_HEIGHT: i32 = 18;
pub(crate) const QUESTION_POINT: i32 = 28;
pub(crate) const SELECT_SKEW: i32 = 24;
/// Font size of a node label. The serializer writes the stylesheet from this,
/// so measurement and rendering cannot disagree.
pub(crate) const LABEL_FONT: i32 = 14;
/// Font size of a connection label, written into the stylesheet the same way.
pub(crate) const CONNECTION_LABEL_FONT: i32 = 12;
/// Font size of an authored question-branch description.
pub(crate) const BRANCH_LABEL_FONT: i32 = 14;
/// Baseline-to-baseline distance between the lines of a node label.
pub(crate) const LINE_HEIGHT: i32 = 18;
/// Baseline-to-baseline distance between the lines of a connection label.
pub(crate) const CONNECTION_LINE_HEIGHT: i32 = 14;
/// Baseline-to-baseline distance between question-branch description lines.
pub(crate) const BRANCH_LINE_HEIGHT: i32 = 18;
/// Width of the halo a connection label paints behind itself to stay readable
/// where it crosses a connection. Also written into the stylesheet.
pub(crate) const CONNECTION_LABEL_HALO: i32 = 5;
/// Distance between two horizontal runs sharing one row gap.
const LANE: i32 = 20;
/// Text budget inside a rectangular node.
const NODE_LABEL_WIDTH: i32 = NODE_WIDTH - 32;
/// Branch icons lose horizontal space to their slanted sides.
const BRANCH_LABEL_WIDTH: i32 = NODE_WIDTH - 80;
/// Text budget inside a case icon.
const CASE_LABEL_WIDTH: i32 = CASE_WIDTH - 32;

pub(crate) struct Scene {
    pub(crate) width: i32,
    pub(crate) height: i32,
    /// The structure this scene places, as the validated model settled it.
    pub(crate) topology: Topology,
    /// The checked arrangement it realizes. Ranks, columns, corridors, and
    /// contours are decisions, not suggestions.
    pub(crate) arrangement: Arrangement,
    /// The strings its nodes, exits, and junctions show.
    pub(crate) captions: Captions,
    pub(crate) nodes: Vec<Node>,
    pub(crate) parameters: Option<ParameterPanel>,
    pub(crate) connections: Vec<Connection>,
    pub(crate) labels: Vec<Label>,
}

/// The flow parameters shown beside start, outside the control-flow topology.
pub(crate) struct ParameterPanel {
    pub(crate) parameters: Vec<String>,
    pub(crate) x: i32,
    pub(crate) y: i32,
    pub(crate) width: i32,
    pub(crate) height: i32,
    pub(crate) lines: Vec<String>,
}

/// One placed node. Its role and caption stay in the topology; only geometry
/// and the wrapped caption lines live here.
pub(crate) struct Node {
    pub(crate) id: NodeId,
    pub(crate) x: i32,
    pub(crate) y: i32,
    pub(crate) width: i32,
    pub(crate) height: i32,
    pub(crate) lines: Vec<String>,
}

/// One point of the diagram, in pixels. The model's own check reads the same
/// type over its abstract grid, so the crossing rules of `geometry` decide for
/// both.
pub(crate) use kaalang_model::geometry::Point;

/// Closes every row the finished drawing leaves empty.
///
/// The arrangement keeps a rank of its own for each iteration tail, because a
/// return leaves one horizontally and anything else on that rank stands in its
/// way. RFC 0003 §2 then lets the return leave at a side exit instead, and
/// `compact_returns` takes that offer: the rank's band is left holding nothing
/// but the connections passing through it, while the two node rows around it
/// sit a row further apart for no reason. Closing the band shortens those
/// connections and lifts everything below it, which is what puts two
/// consecutive blocks one gap apart again.
///
/// Rows are closed from the bottom up, so the bands above one keep the
/// positions `rows` recorded. A band counts as empty only when no route has a
/// point in it — a junction drawn on its own rank has one there, and so does
/// every bend and endpoint — so closing it cannot bring two of them together.
/// `route::verify` has the final word all the same.
fn close_unused_rows(scene: &mut Scene, rows: &Rows) {
    for row in (0..rows.height.len()).rev() {
        let (top, bottom) = (rows.top[row], rows.top[row + 1]);
        if rows.height[row] != 0 || bottom <= top {
            continue;
        }
        if scene
            .connections
            .iter()
            .flat_map(|connection| &connection.points)
            .any(|point| point.y >= top && point.y < bottom)
        {
            continue;
        }
        shift(scene, bottom, top - bottom);
        if route::verify(scene).is_some() {
            shift(scene, top, bottom - top);
        }
    }
}

/// Moves every node, panel, and route point at or below `from` by `delta`.
fn shift(scene: &mut Scene, from: i32, delta: i32) {
    for connection in &mut scene.connections {
        for point in &mut connection.points {
            if point.y >= from {
                point.y += delta;
            }
        }
    }
    for node in &mut scene.nodes {
        if node.y >= from {
            node.y += delta;
        }
    }
    if let Some(parameters) = &mut scene.parameters
        && parameters.y >= from
    {
        parameters.y += delta;
    }
}

/// The rails of the loops nested inside this one, as already drawn.
fn nested_rails(scene: &Scene, index: usize) -> Vec<(usize, i32)> {
    let loop_ = scene.topology.loops[index];
    scene
        .topology
        .loops
        .iter()
        .enumerate()
        .filter(|&(other, _)| other != index)
        .filter_map(|(_, other)| {
            // The second point of a return is the column it climbs in; its
            // first and last are the tail and the entry.
            let rail = scene
                .connections
                .iter()
                .find(|edge| edge.source == Source::Junction(other.tail))?
                .points
                .get(1)?
                .x;
            Some((other.header, rail))
        })
        .filter(|(header, _)| *header != loop_.header)
        .collect()
}

/// One routed connection. It owns no label: a hand-over belongs to the exit it
/// leaves and a capture to the node it reaches.
pub(crate) struct Connection {
    pub(crate) source: Source,
    pub(crate) destination: Destination,
    pub(crate) points: Vec<Point>,
}

/// One placed label, wrapped during layout so the canvas can be sized around
/// it. `at` is the left end of the first line's baseline, so the serializer
/// writes the block without deciding where it sits.
pub(crate) struct Label {
    pub(crate) kind: LabelKind,
    pub(crate) lines: Vec<String>,
    pub(crate) at: Point,
}

#[derive(Clone, Copy)]
pub(crate) enum LabelKind {
    Wire,
    Branch,
}

impl LabelKind {
    pub(crate) const fn font_size(self) -> i32 {
        match self {
            Self::Wire => CONNECTION_LABEL_FONT,
            Self::Branch => BRANCH_LABEL_FONT,
        }
    }

    pub(crate) const fn line_height(self) -> i32 {
        match self {
            Self::Wire => CONNECTION_LINE_HEIGHT,
            Self::Branch => BRANCH_LINE_HEIGHT,
        }
    }
}

/// Shortens each sole tail arrival and brings its return in with it, keeping
/// the side and lane the arrangement chose. RFC 0003 §2 prefers turning
/// upward at a side exit over descending to the tail's row and returning; a
/// shortening that breaks RFC 0002 §8 is dropped, leaving the arrangement the
/// arrangement already made valid.
///
/// Only `route::verify` decides that, and only over the connections: the
/// labels are placed after this and checked then, and where a label bounds a
/// shortening it is `compact_arrival` that reads its rectangle.
fn compact_returns(scene: &mut Scene, model: &SemanticModel) {
    let gap = vertical_gap(scene);
    let labels = label::place_labels(scene);
    for index in (0..scene.topology.loops.len()).rev() {
        let loop_ = scene.topology.loops[index];
        let arrivals = scene
            .connections
            .iter()
            .enumerate()
            .filter(|(_, edge)| edge.destination == Destination::Junction(loop_.tail))
            .map(|(edge, _)| edge)
            .collect::<Vec<_>>();
        let ([arrival], Some(back)) = (
            arrivals.as_slice(),
            scene
                .connections
                .iter()
                .position(|edge| edge.source == Source::Junction(loop_.tail)),
        ) else {
            continue;
        };
        let arrival = *arrival;
        let end = *scene.connections[back]
            .points
            .last()
            .expect("a return reaches its entry");
        let Some(compact) =
            compact_arrival(&scene.connections[arrival].points, gap, end.y, &labels)
        else {
            continue;
        };
        let kept = (
            scene.connections[arrival].points.clone(),
            scene.connections[back].points.clone(),
        );
        let from = *compact.last().expect("a shortened arrival has a route");
        scene.connections[arrival].points = compact;
        let nested = nested_rails(scene, index);
        let aside = route::contour_x(scene, model, index, from, end, &nested);
        scene.connections[back].points = straighten_return(from, aside, end);
        if route::verify(scene).is_some() {
            scene.connections[arrival].points = kept.0;
            scene.connections[back].points = kept.1;
        }
    }
}

fn straighten_return(from: Point, aside: i32, end: Point) -> Vec<Point> {
    let mut points = vec![
        from,
        Point {
            x: aside,
            y: from.y,
        },
        Point { x: aside, y: end.y },
        end,
    ];
    points.dedup();
    points
}

/// A sole arrival can turn earlier without reserving a node's row or column.
/// Side exits keep enough horizontal room for labels above them.
fn compact_arrival(
    points: &[Point],
    gap: i32,
    entry_y: i32,
    labels: &[Label],
) -> Option<Vec<Point>> {
    let [.., bend, end] = points else {
        return None;
    };
    let mut compact = points.to_vec();
    let y = bend.y + if points.len() == 2 { gap } else { 0 };
    if bend.x == end.x && y < end.y {
        compact.pop();
        if y != bend.y {
            compact.push(Point { x: end.x, y });
        }
    }
    if let [start, end] = compact.as_mut_slice()
        && start.y == end.y
        && start.x < end.x
    {
        let right = labels
            .iter()
            .map(label_rect)
            .filter(|&(_, top, _, bottom)| bottom > entry_y && top < end.y)
            .map(|(_, _, right, _)| right)
            .fold(start.x + LANE, i32::max);
        end.x = end.x.min(right);
    }
    (compact != points).then_some(compact)
}

/// The authored flow header without its parameters and return type.
pub(crate) fn start_text(source: &str, signature: &Signature) -> String {
    let start = signature.span().byte_range().start;
    let end = signature.paren_token.span.open().byte_range().start;
    let function = signature.fn_token.span.byte_range();
    let mut header = source[start..end].to_owned();
    header.replace_range(function.start - start..function.end - start, "");
    let mut authored = collapsed_text(&header);
    if let Some(where_clause) = &signature.generics.where_clause {
        authored.push(' ');
        authored.push_str(&collapsed(source, where_clause.span()));
    }
    authored
}

/// The authored flow parameters, one per panel row and without separating commas.
pub(crate) fn parameter_text(source: &str, signature: &Signature) -> Vec<String> {
    signature
        .inputs
        .iter()
        .map(|parameter| collapsed(source, parameter.span()))
        .collect()
}

/// The end node's caption: the authored return type, or `()` when absent.
pub(crate) fn return_text(source: &str, output: &ReturnType) -> String {
    match output {
        ReturnType::Default => "()".to_owned(),
        ReturnType::Type(_, ty) => collapsed(source, ty.span()),
    }
}

/// The authored text under a span with every whitespace run collapsed to one space.
fn collapsed(source: &str, span: proc_macro2::Span) -> String {
    collapsed_range(source, span.byte_range())
}

fn collapsed_range(source: &str, range: std::ops::Range<usize>) -> String {
    collapsed_text(&source[range])
}

fn collapsed_text(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Lays out one validated flow, or reports that this layout could not route its
/// connections under RFC 0002 §8.
pub(crate) fn layout(
    model: &SemanticModel,
    start: &str,
    parameters: &[String],
    return_type: &str,
) -> Result<Scene, String> {
    let mut scene = Scene {
        width: 0,
        height: 0,
        nodes: Vec::new(),
        parameters: parameter_panel(parameters),
        connections: Vec::new(),
        labels: Vec::new(),
        captions: captions::derive(model, start, return_type),
        arrangement: kaalang_model::construct(&model.flow, &model.merges, &model.topology)
            .map_err(|error| error.to_string())?,
        topology: model.topology.clone(),
    };

    scene.nodes = nodes(&scene);
    let rows = scene.rows();
    scene.lift(&rows);
    scene.connections = route::emit(&scene, &rows);
    scene
        .connections
        .extend(route::returns(&scene, model, &rows));
    // The final word on RFC 0002 §8. The arrangement is checked in abstract
    // ranks and columns; this reads the emitted geometry, so it catches
    // anything the abstract check does not model. A disagreement means the
    // realization is wrong, not that another arrangement should be tried.
    if let Some(reason) = route::verify(&scene) {
        return Err(reason);
    }
    compact_returns(&mut scene, model);
    end::adjust(&mut scene);
    close_unused_rows(&mut scene, &rows);
    scene.labels = label::place_labels(&scene);
    scene.indent();
    scene.fit();
    // The same final word for the labels. It waits for `indent` and `fit`
    // because those settle the coordinates and the canvas the labels are
    // checked against.
    if let Some(reason) = label::verify(&scene) {
        return Err(reason);
    }

    Ok(scene)
}

/// Every node at its column, with its own dimensions. Rows are added once the
/// row gaps are known, so the vertical position waits for the routing plan.
fn nodes(scene: &Scene) -> Vec<Node> {
    scene
        .topology
        .nodes
        .iter()
        .map(|node| {
            let (width, height, lines) = node_dimensions(node.kind, scene.captions.label(node.id));
            Node {
                id: node.id,
                x: column_x(scene.column(Vertex::Node(node.id))),
                y: 0,
                width,
                height,
                lines,
            }
        })
        .collect()
}

/// Top edge and height of every row, in row order. A junction row carries no
/// node, so it is only the lane its routes meet in.
///
/// `top` and `capture_space` carry one entry past the last row, the bottom edge
/// of the diagram, so that the row gap below a row is always addressable as
/// `row + 1`. The trailing entry has no node, so it needs no capture space.
pub(super) struct Rows {
    top: Vec<i32>,
    height: Vec<i32>,
    /// Per rank gap, how many lanes its sideways runs occupy.
    lanes: Vec<usize>,
    /// Space between the last horizontal arrival and the following node row;
    /// zero when the row contains only junctions.
    capture_space: Vec<i32>,
}

impl Rows {
    pub(super) fn lanes_in(&self, gap: usize) -> usize {
        self.lanes.get(gap).copied().unwrap_or(0)
    }

    /// Packs horizontal runs toward the following row, leaving the spare room
    /// below the producers rather than pressing the merge against their exits.
    pub(super) fn lane_y(&self, gap: usize, lane: usize, lanes: usize) -> i32 {
        self.top[gap + 1] - self.capture_space[gap + 1] - (lanes - lane - 1) as i32 * LANE
    }

    /// A junction row carries no node, so the routes that meet there meet on
    /// the row's own line.
    pub(super) fn junction_y(&self, row: usize) -> i32 {
        self.top[row]
    }
}

impl Scene {
    fn rows(&self) -> Rows {
        let ranks = self.arrangement.ranks;
        let gap = vertical_gap(self);
        let mut height = vec![0; ranks];
        let mut capture_space = vec![0; ranks + 1];
        for node in &self.nodes {
            let row = self.rank(Vertex::Node(node.id));
            height[row] = height[row].max(node.height);
            capture_space[row] = gap;
        }
        if let Some(parameters) = &self.parameters {
            let row = self.rank(Vertex::Node(NodeId::Start));
            height[row] = height[row].max(parameters.height);
        }
        let lanes = self.arrangement.gap_lanes.clone();
        let mut top = Vec::with_capacity(ranks + 1);
        let mut next = MARGIN;
        for (row, own) in height.iter().enumerate() {
            top.push(next);
            let count = lanes.get(row).copied().unwrap_or(0) as i32;
            let routing = if count == 0 {
                0
            } else {
                gap + (count - 1) * LANE + capture_space[row + 1]
            };
            next += own + gap.max(routing);
        }
        top.push(next);

        Rows {
            top,
            height,
            lanes,
            capture_space,
        }
    }

    /// The rank and column the arrangement gave one vertex.
    pub(super) fn rank(&self, vertex: Vertex) -> usize {
        self.arrangement.rank[&vertex]
    }

    pub(super) fn column(&self, vertex: Vertex) -> i32 {
        self.arrangement.column[&vertex]
    }

    /// Centres every node on its row, once the rows have their heights.
    fn lift(&mut self, rows: &Rows) {
        for node in &mut self.nodes {
            let row = self.arrangement.rank[&Vertex::Node(node.id)];
            node.y = rows.top[row] + rows.height[row] / 2;
        }
        let start = self.node(NodeId::Start);
        let (x, y, width) = (start.x, start.y, start.width);
        if let Some(parameters) = &mut self.parameters {
            parameters.x = x + width / 2 + PARAMETER_PANEL_GAP + parameters.width / 2;
            parameters.y = y;
        }
    }

    pub(crate) fn is_back_edge(&self, connection: &Connection) -> bool {
        self.topology.back_edges.iter().any(|edge| {
            edge.source == connection.source && edge.destination == connection.destination
        })
    }

    pub(crate) fn node(&self, id: NodeId) -> &Node {
        self.nodes
            .iter()
            .find(|node| node.id == id)
            .expect("every projected node is placed")
    }

    pub(super) const fn bounds(node: &Node) -> (i32, i32, i32, i32) {
        (
            node.x - node.width / 2,
            node.y - node.height / 2,
            node.x + node.width / 2,
            node.y + node.height / 2,
        )
    }

    pub(super) const fn parameter_bounds(parameters: &ParameterPanel) -> (i32, i32, i32, i32) {
        (
            parameters.x - parameters.width / 2,
            parameters.y - parameters.height / 2,
            parameters.x + parameters.width / 2,
            parameters.y + parameters.height / 2,
        )
    }

    pub(super) fn top_anchor(&self, id: NodeId) -> Point {
        let node = self.node(id);
        Point {
            x: node.x,
            y: node.y - node.height / 2,
        }
    }

    /// Where one exit's connections leave the node boundary.
    pub(super) fn exit_anchor(&self, exit: ExitId) -> Point {
        let node = self.node(exit.node);
        match self.topology.node(exit.node).kind {
            NodeKind::Question => question::exit_anchor(
                node,
                exit.branch.expect("a question exit belongs to a branch"),
            ),
            _ => Point {
                x: node.x,
                y: node.y + node.height / 2,
            },
        }
    }

    /// Slides the whole scene right when a route left the diagram on the left
    /// to get out of another's way, so the canvas still starts at the origin.
    fn indent(&mut self) {
        let left = self
            .connections
            .iter()
            .flat_map(|connection| &connection.points)
            .map(|point| point.x)
            .min()
            .unwrap_or(MARGIN)
            .min(MARGIN);
        if left >= MARGIN {
            return;
        }

        let shift = MARGIN - left;
        for node in &mut self.nodes {
            node.x += shift;
        }
        for connection in &mut self.connections {
            for point in &mut connection.points {
                point.x += shift;
            }
        }
        for label in &mut self.labels {
            label.at.x += shift;
        }
        if let Some(parameters) = &mut self.parameters {
            parameters.x += shift;
        }
    }

    fn fit(&mut self) {
        let (mut right, mut bottom) = (0, 0);
        for node in &self.nodes {
            right = right.max(node.x + node.width / 2);
            bottom = bottom.max(node.y + node.height / 2);
        }
        for connection in &self.connections {
            for point in &connection.points {
                right = right.max(point.x);
                bottom = bottom.max(point.y);
            }
        }
        for label in &self.labels {
            let (.., label_right, label_bottom) = label_rect(label);
            right = right.max(label_right);
            bottom = bottom.max(label_bottom);
        }
        if let Some(parameters) = &self.parameters {
            let (.., parameters_right, parameters_bottom) = Self::parameter_bounds(parameters);
            right = right.max(parameters_right);
            bottom = bottom.max(parameters_bottom);
        }
        self.width = right + MARGIN;
        self.height = bottom + MARGIN;
    }
}

fn parameter_panel(parameters: &[String]) -> Option<ParameterPanel> {
    if parameters.is_empty() {
        return None;
    }
    let lines = parameters
        .iter()
        .flat_map(|parameter| wrap_text(parameter, PARAMETER_LABEL_WIDTH, LABEL_FONT))
        .collect::<Vec<_>>();
    Some(ParameterPanel {
        parameters: parameters.to_vec(),
        x: 0,
        y: 0,
        width: PARAMETER_PANEL_WIDTH,
        height: 30 + lines.len() as i32 * LINE_HEIGHT,
        lines,
    })
}

fn column_x(column: i32) -> i32 {
    MARGIN + NODE_WIDTH / 2 + column * COLUMN_WIDTH
}

fn node_dimensions(kind: NodeKind, label: &str) -> (i32, i32, Vec<String>) {
    match kind {
        NodeKind::Start | NodeKind::End => capsule_dimensions(label),
        NodeKind::Action => action::dimensions(label),
        NodeKind::Question | NodeKind::Select => {
            block_dimensions(label, NODE_WIDTH, BRANCH_LABEL_WIDTH, 72)
        }
        NodeKind::Case => choice::case_dimensions(label),
    }
}

/// Fits each line inside the curved ends, not just the capsule's bounding box.
/// SVG clamps the horizontal radius once the capsule grows taller than wide,
/// so those tall capsules use the corresponding ellipse bound instead.
fn capsule_dimensions(label: &str) -> (i32, i32, Vec<String>) {
    let (width, mut height, lines) = block_dimensions(label, NODE_WIDTH, NODE_LABEL_WIDTH, 58);
    let first_baseline = 15 - lines.len() as i32 * LINE_HEIGHT / 2;
    for (index, line) in lines.iter().enumerate() {
        let line_width = text::text_width(&line.graphemes(true).collect::<Vec<_>>(), LABEL_FONT);
        let x = f64::from(line_width) / 2.0 + 4.0;
        let baseline = first_baseline + index as i32 * LINE_HEIGHT;
        let y = f64::from(
            (baseline - LABEL_FONT)
                .abs()
                .max((baseline + LABEL_FONT / 3).abs()),
        ) + 4.0;
        let inset = f64::from(width) / 2.0 - x;
        // The corner fits when (radius - inset)^2 + y^2 <= radius^2.
        let required = if y <= inset {
            2.0 * y
        } else {
            inset + y * y / inset
        };
        let required = if required <= f64::from(width) {
            required
        } else {
            2.0 * y / (1.0 - (2.0 * x / f64::from(width)).powi(2)).sqrt()
        };
        height = height.max(2 * (required / 2.0).ceil() as i32);
    }
    (width, height, lines)
}

fn block_dimensions(
    label: &str,
    width: i32,
    budget: i32,
    minimum_height: i32,
) -> (i32, i32, Vec<String>) {
    let lines = wrap_text(label, budget, LABEL_FONT);
    let height = minimum_height.max(30 + lines.len() as i32 * LINE_HEIGHT);
    (width, height, lines)
}
