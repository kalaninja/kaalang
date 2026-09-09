//! Places the visual topology on rows and columns, routes its connections, and
//! positions the labels its exits and nodes own.

use std::collections::BTreeMap;

use kaalang_model::SemanticModel;
use syn::{ReturnType, Signature, spanned::Spanned};
use unicode_segmentation::UnicodeSegmentation;

use crate::topology::{self, Destination, ExitId, NodeId, NodeKind, Source, Topology, Vertex};

mod action;
mod choice;
mod label;
mod place;
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
    /// The projection this scene places. Node roles and the labels the exits
    /// and nodes own are read from here rather than copied.
    pub(crate) topology: Topology,
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Point {
    pub(crate) x: i32,
    pub(crate) y: i32,
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

/// The end node's caption: the authored return type preceded by `->`, and
/// `-> ()` when the function declares none.
pub(crate) fn return_text(source: &str, output: &ReturnType) -> String {
    match output {
        ReturnType::Default => "-> ()".to_owned(),
        ReturnType::Type(..) => collapsed(source, output.span()),
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
    let topology = topology::project(model, start, return_type);
    let attempts = 4 * topology.connections.len() + 8;
    let mut scene = Scene {
        width: 0,
        height: 0,
        nodes: Vec::new(),
        parameters: parameter_panel(parameters),
        connections: Vec::new(),
        labels: Vec::new(),
        topology,
    };

    // Routing asks for more room rather than settling for a crossing: a lower
    // row for a destination, or a column of its own for a run. Both only ever
    // add room, so the loop cannot cycle.
    let mut delays = BTreeMap::new();
    let mut shapes: BTreeMap<usize, route::Shape> = BTreeMap::new();
    let mut blocking = String::new();
    for _ in 0..attempts {
        let placement = place::place(&scene.topology, model, &delays)?;
        scene.nodes = nodes(&scene.topology, &placement);
        let blocked = match route::plan(&scene, &placement, &shapes) {
            Ok(plan) => {
                let rows = scene.rows(&placement, &plan);
                scene.lift(&placement, &rows);
                scene.connections = route::emit(&scene, &placement, &plan, &rows);
                // The final word on RFC 0002 §8. The planner reasons over
                // columns and lanes; this reads the emitted geometry, so it
                // catches anything the planner does not model. It is a refusal
                // rather than another rung of the ladder because the two are
                // not proven equivalent: a disagreement means the planner is
                // wrong, and no amount of extra room would make it right.
                if let Some(reason) = route::verify(&scene) {
                    return Err(reason);
                }
                scene.labels = label::place_labels(&scene);
                scene.indent();
                scene.fit();
                // The same final word for the labels. It waits for `indent`
                // and `fit` because those settle the coordinates and the
                // canvas the labels are checked against.
                if let Some(reason) = label::verify(&scene) {
                    return Err(reason);
                }

                return Ok(scene);
            }
            Err(blocked) => blocked,
        };
        // Each blocked run climbs the same ladder: turn sideways later, then
        // take a column of its own, then drop another row. Every rung only adds
        // room, so the ladder ends.
        let route::Blocked { connection, reason } = blocked;
        blocking = reason;
        let wire = scene.topology.connections[connection];
        let destination = wire.destination;
        let span = placement.row(destination) - placement.row(Vertex::from(wire.source));
        // A run into a junction already descends in its own column, so
        // deferring it changes nothing; it climbs straight to the next rung.
        let into_junction = matches!(wire.destination, Destination::Junction(_));
        let shape = shapes.entry(connection).or_default();
        match *shape {
            // A run confined to one row gap has nowhere else to turn, so its
            // destination drops a row and the run gains a gap of its own.
            _ if span < 2 => *delays.entry(destination).or_insert(0) += 1,
            route::Shape::Direct if into_junction => *shape = route::Shape::Aside,
            route::Shape::Direct => *shape = route::Shape::Deferred,
            route::Shape::Deferred => *shape = route::Shape::Aside,
            route::Shape::Aside => *delays.entry(destination).or_insert(0) += 1,
        }
    }

    Err(format!("routing attempts exhausted: {blocking}"))
}

/// Every node at its column, with its own dimensions. Rows are added once the
/// row gaps are known, so the vertical position waits for the routing plan.
fn nodes(topology: &Topology, placement: &place::Placement) -> Vec<Node> {
    topology
        .nodes
        .iter()
        .map(|node| {
            let (width, height, lines) = node_dimensions(node.kind, &node.label);
            Node {
                id: node.id,
                x: column_x(placement.column(Vertex::Node(node.id))),
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
/// `row + 1`. No node's capture claims that trailing entry, so it keeps the
/// default a row without a capture label would have.
pub(super) struct Rows {
    top: Vec<i32>,
    height: Vec<i32>,
    /// Space between the last horizontal arrival and the following node row.
    capture_space: Vec<i32>,
}

impl Rows {
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
    fn rows(&self, placement: &place::Placement, plan: &route::Plan) -> Rows {
        let mut height = vec![0; placement.rows];
        let mut capture_space = vec![LANE; placement.rows + 1];
        for node in &self.nodes {
            let row = placement.row(Vertex::Node(node.id));
            height[row] = height[row].max(node.height);
            capture_space[row] =
                capture_space[row].max(label::capture_space(&self.topology.capture_label(node.id)));
        }
        if let Some(parameters) = &self.parameters {
            let row = placement.row(Vertex::Node(NodeId::Start));
            height[row] = height[row].max(parameters.height);
        }
        let gap = vertical_gap(&self.topology);
        let mut top = Vec::with_capacity(placement.rows + 1);
        let mut next = MARGIN;
        for (row, height) in height.iter().enumerate() {
            top.push(next);
            let lanes = plan.lanes_in(row) as i32;
            let routing = if lanes == 0 {
                0
            } else {
                (lanes + 1) * LANE + capture_space[row + 1]
            };
            next += height + gap.max(routing);
        }
        top.push(next);

        Rows {
            top,
            height,
            capture_space,
        }
    }

    /// Centres every node on its row, once the rows have their heights.
    fn lift(&mut self, placement: &place::Placement, rows: &Rows) {
        for node in &mut self.nodes {
            let row = placement.row(Vertex::Node(node.id));
            node.y = rows.top[row] + rows.height[row] / 2;
        }
        let start = self.node(NodeId::Start);
        let (x, y, width) = (start.x, start.y, start.width);
        if let Some(parameters) = &mut self.parameters {
            parameters.x = x + width / 2 + PARAMETER_PANEL_GAP + parameters.width / 2;
            parameters.y = y;
        }
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

fn column_x(column: usize) -> i32 {
    MARGIN + NODE_WIDTH / 2 + column as i32 * COLUMN_WIDTH
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
