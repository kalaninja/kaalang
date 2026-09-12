//! Places the visual topology on rows and columns, routes its connections, and
//! positions the labels its exits and nodes own.

use std::collections::BTreeSet;

use kaalang_model::topology::{Destination, ExitId, NodeId, NodeKind, Source, Topology, Vertex};
use kaalang_model::{Arrangement, SemanticModel, Side};
use syn::{ReturnType, Signature, spanned::Spanned};

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
    /// How far past a body's edge a return's rail reaches, on the left and on
    /// the right. `column_width` holds a gap wide enough for both at once.
    reach: (i32, i32),
    /// Extra room between columns, asked for by a previous pass whose returns
    /// and labels wanted the same gap. Zero for almost every diagram.
    slack: i32,
    /// What each loop's body draws, as the model counted it when it placed the
    /// return: one entry per `topology.loops`. A presentation measures the
    /// same body rather than a set of its own.
    bodies: Vec<BTreeSet<Vertex>>,
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

/// The rails of the loops nested inside this one, as already drawn.
fn nested_rails(scene: &Scene, index: usize) -> Vec<(usize, i32)> {
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
        .collect()
}

/// Shortens each sole tail arrival and brings its return in with it, keeping
/// the side and lane the arrangement chose. RFC 0003 §2 prefers turning
/// upward at a side exit over descending to the tail's row and returning; a
/// shortening that breaks RFC 0002 §8 is dropped, leaving the arrangement the
/// arrangement already made valid.
///
/// Only `route::verify` decides that, and only over the connections; the
/// labels are placed after this and checked then. Where a label bounds a
/// shortening it is `compact_arrival` that reads its rectangle, and it reads
/// the labels of the scene as it stands: each accepted shortening moves the
/// route its labels hang from, so they are placed again for the next loop.
///
/// It can move the climb: `compact_arrival` shortens a horizontal arrival, and
/// the shortened end is what seeds the body extent `contour_x` measures from.
/// That is why the return is re-emitted here from the same rows the loop
/// spans, and why `route::verify` has the last word — it holds the re-emitted
/// climb outside the body like any other.
fn compact_returns(scene: &mut Scene, model: &SemanticModel) {
    let gap = vertical_gap(scene);
    for index in (0..scene.topology.loops.len()).rev() {
        let labels = label::place_labels(scene);
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

/// Steps a return further out until its climb clears the labels it would
/// otherwise strike through.
///
/// Labels are placed against the routes, so where they end up is not known
/// when the returns are drawn, and a label hanging off the body's outermost
/// route reaches past the rail beside it. Moving out only adds room between
/// the climb and the body, so RFC 0002 §8 holds either way and `route::verify`
/// confirms it; a step it refuses is dropped and the climb stays where it was.
fn clear_labels(scene: &mut Scene) -> i32 {
    let mut wanted = 0;
    // Innermost first: an enclosing rail is measured from the one it encloses,
    // so it has to follow it out rather than be stepped into.
    for index in (0..scene.topology.loops.len()).rev() {
        // The rail and every rail outside it on the same side. Moving one
        // alone would close the lane between them, which the contour rule
        // refuses — rightly, so the whole chain moves together.
        let side = scene.arrangement.contours[index].side;
        let rail = |scene: &Scene, other: usize| {
            let tail = scene.topology.loops[other].tail;
            scene
                .connections
                .iter()
                .position(|edge| edge.source == Source::Junction(tail))
        };
        let Some(back) = rail(scene, index) else {
            continue;
        };
        let chain = std::iter::once(back)
            .chain(
                (0..scene.topology.loops.len())
                    .filter(|&other| {
                        other != index
                            && scene.arrangement.contours[other].side == side
                            && scene.bodies[other]
                                .contains(&Vertex::Junction(scene.topology.loops[index].entry))
                    })
                    .filter_map(|other| rail(scene, other)),
            )
            .collect::<Vec<_>>();
        let step = match side {
            Side::Left => -LANE,
            Side::Right => LANE,
        };
        let struck = |scene: &Scene| {
            scene
                .labels
                .iter()
                .map(label::label_rect)
                .filter(|rect| {
                    scene.connections[back]
                        .points
                        .windows(2)
                        .any(|segment| route::enters(segment[0], segment[1], *rect))
                })
                .map(|rect| rect.2)
                .max()
        };
        // A single label can span several lanes. Past the outermost label no
        // climb can strike text; a collision left there is on a horizontal
        // run, which stepping farther out cannot clear.
        let rail = scene.connections[back].points[1].x;
        let outermost = scene.labels.iter().map(label_rect).fold(
            rail,
            |outer, (left, _, right, _)| match side {
                Side::Left => outer.min(left),
                Side::Right => outer.max(right),
            },
        );
        for _ in 0..rail.abs_diff(outermost).div_ceil(LANE.unsigned_abs()) {
            if struck(scene).is_none() {
                break;
            }
            let kept = chain
                .iter()
                .map(|&edge| scene.connections[edge].points.clone())
                .collect::<Vec<_>>();
            for &edge in &chain {
                for point in &mut scene.connections[edge].points[1..3] {
                    point.x += step;
                }
            }
            if route::verify(scene).is_some() {
                for (&edge, points) in chain.iter().zip(kept) {
                    scene.connections[edge].points = points;
                }
                break;
            }
        }
        // Still struck, and nowhere left to step: the gap itself is too
        // narrow. Ask for enough that the rail clears the label's far edge
        // once the body it hangs off has moved out with its column.
        if let Some(edge) = struck(scene) {
            let rail = scene.connections[back].points[1].x;
            wanted = wanted.max(((edge - rail).abs() + LANE).max(LANE));
        }
    }
    wanted
}

fn straighten_return(from: Point, aside: i32, end: Point) -> Vec<Point> {
    let mut points = route::return_points(from, aside, end);
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
    collapsed_text(&source[span.byte_range()])
}

fn collapsed_text(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// How far past its body's edge each return's rail can reach, on the left and
/// on the right.
///
/// A rail sits `(lane + 1)` lanes past the body it climbs, and a lane past any
/// rail already drawn beside a body nested in that one — so a reach grows
/// along the deepest chain of nested loops rather than with their number.
/// `route::contour_x` builds the rails that way; this is the same arithmetic
/// before any pixel is placed, which is what lets `column_width` use it.
fn contour_reach(model: &SemanticModel) -> (i32, i32) {
    let loops = &model.topology.loops;
    let mut reach = vec![0; loops.len()];
    // Innermost first: `topology.loops` runs outermost first.
    for index in (0..loops.len()).rev() {
        let end = model.flow.blocks[loops[index].header]
            .loop_end
            .expect("a loop owns a body");
        let body = loops[index].header + 1..end;
        let nested = loops
            .iter()
            .enumerate()
            .filter(|(_, other)| body.contains(&other.header))
            .map(|(other, _)| reach[other] + LANE)
            .max()
            .unwrap_or(0);
        let lane =
            i32::try_from(model.arrangement.contours[index].lane).unwrap_or(i32::MAX / LANE) + 1;
        reach[index] = nested.max(lane * LANE);
    }
    let widest = |want| {
        (0..loops.len())
            .filter(|&index| model.arrangement.contours[index].side == want)
            .map(|index| reach[index])
            .max()
            .unwrap_or(0)
    };
    (widest(Side::Left), widest(Side::Right))
}

/// Lays out one validated flow, or reports that this layout could not route its
/// connections under RFC 0002 §8.
pub(crate) fn layout(
    model: &SemanticModel,
    start: &str,
    parameters: &[String],
    return_type: &str,
) -> Result<Scene, String> {
    // A label is wrapped to fit the standard gap beside its column, and a
    // return climbs in that same gap. Where the two want it at once — a label
    // reaching rightward from one column past the rail of the loop in the
    // next — no rail position clears it, because stepping out goes further
    // into the label and stepping in goes into the body. The room has to come
    // from the columns, and how much is only known once the labels are
    // placed, so the layout is taken again with the gap the last one asked
    // for. These presentation retries are bounded; exhaustion is a renderer
    // error, not evidence that the model's topology is impossible.
    let mut slack = 0;
    for _ in 0..4 {
        match attempt(model, start, parameters, return_type, slack) {
            Ok(scene) => return Ok(scene),
            Err(Blocked::Refused(reason)) => return Err(reason),
            Err(Blocked::Narrow(wanted)) => slack += wanted,
        }
    }
    attempt(model, start, parameters, return_type, slack).map_err(|blocked| match blocked {
        Blocked::Refused(reason) => reason,
        Blocked::Narrow(_) => "a loop return and a connection label want the same gap".to_owned(),
    })
}

/// Why one pass of the layout produced no scene.
enum Blocked {
    /// This pass breaks a geometry rule other than the gap conflict below.
    Refused(String),
    /// A return could not be drawn clear of a label; the columns need this
    /// much more room between them.
    Narrow(i32),
}

fn attempt(
    model: &SemanticModel,
    start: &str,
    parameters: &[String],
    return_type: &str,
    slack: i32,
) -> Result<Scene, Blocked> {
    let mut scene = Scene {
        slack,
        width: 0,
        height: 0,
        nodes: Vec::new(),
        parameters: parameter_panel(parameters),
        connections: Vec::new(),
        labels: Vec::new(),
        captions: captions::derive(model, start, return_type),
        reach: contour_reach(model),
        bodies: model
            .topology
            .loops
            .iter()
            .map(|loop_| model.body_vertices(loop_.header))
            .collect(),
        arrangement: model.arrangement.clone(),
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
        return Err(Blocked::Refused(reason));
    }
    compact_returns(&mut scene, model);
    end::adjust(&mut scene);
    close_unused_rows(&mut scene, &rows);
    scene.labels = label::place_labels(&scene);
    let wanted = clear_labels(&mut scene);
    if wanted > 0 {
        return Err(Blocked::Narrow(wanted));
    }
    scene.indent();
    scene.fit();
    // Check the complete result after all transformations, including the
    // coordinates and canvas that `indent` and `fit` settle.
    if let Some(reason) = route::verify(&scene).or_else(|| label::verify(&scene)) {
        return Err(Blocked::Refused(reason));
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
                x: scene.column_x(scene.column(Vertex::Node(node.id))),
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

    /// The centre of one column.
    ///
    /// Columns stand `COLUMN_WIDTH` apart unless the arrangement climbs a
    /// return in a lane deeper than the space that leaves beside a node. Then
    /// every column moves out far enough to hold that lane and one more, which
    /// is how a presentation holds whatever lanes the model allowed.
    ///
    /// ponytail: one width for the whole diagram, so one deep contour widens
    /// every gap; give each gap its own width if a diagram looks stretched.
    pub(super) fn column_x(&self, column: i32) -> i32 {
        MARGIN + NODE_WIDTH / 2 + column * self.column_width()
    }

    /// How far apart two columns stand.
    ///
    /// Wide enough that the deepest rail reaching into a gap from the left and
    /// the deepest reaching into it from the right still leave a lane between
    /// them. `contour_reach` measures both, so a rail pushed out by a chain of
    /// nested returns is held as well as one pushed out by its own lane.
    ///
    /// ponytail: one width for the whole diagram, so one deep contour widens
    /// every gap; give each gap its own width if a diagram looks stretched.
    fn column_width(&self) -> i32 {
        let (left, right) = self.reach;
        COLUMN_WIDTH.max(NODE_WIDTH + left + right + LANE) + self.slack
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
        let line_width = text::text_width(line, LABEL_FONT);
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
