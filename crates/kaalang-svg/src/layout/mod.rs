//! Places the visual topology on rows and columns, routes its connections, and
//! positions the labels its exits and nodes own.

use std::collections::{BTreeMap, BTreeSet};

use kaalang_model::topology::{Destination, ExitId, NodeId, NodeKind, Source, Topology, Vertex};
use kaalang_model::{Arrangement, RunLine, SemanticModel, Side};
use syn::{ReturnType, Signature, spanned::Spanned};

use crate::captions::{self, Captions};

mod action;
mod call;
mod choice;
mod end;
mod label;
mod loop_block;
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
pub(crate) const MERGE_RADIUS: i32 = 4;
/// Leaves the start hand-over label clear beneath the link to the panel.
const PARAMETER_PANEL_GAP: i32 = 96;
const PARAMETER_PANEL_WIDTH: i32 = 240;
const PARAMETER_LABEL_WIDTH: i32 = PARAMETER_PANEL_WIDTH - 32;
const CASE_WIDTH: i32 = 240;
/// How far inside each side of a call node its bar sits.
pub(crate) const CALL_BAR_INSET: i32 = 12;
pub(crate) const CASE_TIP_HEIGHT: i32 = 18;
pub(crate) const QUESTION_POINT: i32 = 28;
pub(crate) const SELECT_SKEW: i32 = 24;
/// Font size of a node label. The serializer writes the stylesheet from this,
/// so measurement and rendering cannot disagree.
pub(crate) const LABEL_FONT: i32 = 14;
/// Secondary captions fit two lines in an expanded cycle's top padding.
pub(crate) const CYCLE_CAPTION_FONT: i32 = 12;
pub(crate) const CYCLE_CAPTION_LINE_HEIGHT: i32 = 14;
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

#[derive(Clone)]
pub(crate) struct Scene {
    pub(crate) width: i32,
    pub(crate) height: i32,
    /// The structure this scene places, as the validated model settled it.
    pub(crate) topology: Topology,
    /// The checked arrangement it realizes. Ranks, columns, corridors, and
    /// contours are decisions, not suggestions.
    pub(crate) arrangement: Arrangement,
    /// How far past a body's edge a back edge rail reaches, on the left and on
    /// the right. `column_width` holds a gap wide enough for both at once.
    reach: (i32, i32),
    /// Extra room between columns, asked for by a previous pass whose back edges
    /// and labels wanted the same gap. Zero for almost every diagram.
    slack: i32,
    /// Routing-only columns need a lane rather than a full node width.
    narrow: bool,
    /// What each cycle's body draws, as the model counted it when it placed the
    /// back edge: one entry per `topology.loops`. A presentation measures the
    /// same body rather than a set of its own.
    bodies: Vec<BTreeSet<Vertex>>,
    /// The owned vertices of every expanded cycle boundary, including cycles
    /// that have no repeating execution and therefore no back edge.
    region_bodies: Vec<BTreeSet<Vertex>>,
    /// The strings its nodes, exits, and junctions show.
    pub(crate) captions: Captions,
    pub(crate) nodes: Vec<Node>,
    pub(crate) parameters: Option<ParameterPanel>,
    pub(crate) connections: Vec<Connection>,
    pub(crate) labels: Vec<Label>,
    pub(crate) loop_regions: Vec<LoopRegion>,
}

/// One described expanded cycle boundary.
#[derive(Clone)]
pub(crate) struct LoopRegion {
    pub(crate) left: i32,
    pub(crate) top: i32,
    pub(crate) right: i32,
    pub(crate) bottom: i32,
    pub(crate) description: String,
    pub(crate) caption: Vec<String>,
    pub(crate) inputs: String,
    pub(crate) outputs: String,
}

impl LoopRegion {
    /// Left, top, right, bottom, like `Scene::bounds`.
    pub(super) const fn bounds(&self) -> (i32, i32, i32, i32) {
        (self.left, self.top, self.right, self.bottom)
    }
}

/// The flow parameters shown beside start, outside the control-flow topology.
#[derive(Clone)]
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
#[derive(Clone)]
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

/// One routed connection. It owns no label: a hand-over belongs to the exit it
/// leaves and a capture to the node it reaches.
#[derive(Clone)]
pub(crate) struct Connection {
    pub(crate) source: Source,
    pub(crate) destination: Destination,
    pub(crate) points: Vec<Point>,
}

/// One placed label, wrapped during layout so the canvas can be sized around
/// it. `at` is the left end of the first line's baseline, so the serializer
/// writes the block without deciding where it sits.
#[derive(Clone)]
pub(crate) struct Label {
    /// The topology vertex whose hand-over, capture, or branch this label describes.
    pub(crate) owner: Vertex,
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

/// Steps a back edge further out until its climb clears labels and leaves a
/// routing lane beside nested cycle frames.
///
/// Labels are placed against the routes, so where they end up is not known
/// when the back edges are drawn, and a label hanging off the body's outermost
/// route reaches past the rail beside it. Moving out only adds room between
/// the climb and the body, so RFC 0002 §8 holds either way and `route::verify`
/// confirms it; a step it refuses is dropped and the climb stays where it was.
#[allow(clippy::too_many_lines)] // One cohesive measure-move-verify pass over each back edge.
fn clear_labels(scene: &mut Scene) -> i32 {
    // The elastic realization already reserves label width on both sides of
    // every back edge column. Moving just one segment would change its corridor.
    if !scene.arrangement.back_routes.is_empty() {
        return 0;
    }
    let mut wanted = 0;
    // Innermost first: an enclosing rail is measured from the one it encloses,
    // so it has to follow it out rather than be stepped into.
    for index in (0..scene.topology.loops.len()).rev() {
        // The rail and every rail outside it on the same side. Moving one
        // alone would close the lane between them, which the contour rule
        // refuses — rightly, so the whole chain moves together.
        let side = scene.arrangement.contours[index].side;
        let Some(back) = scene.back_edge_index(index) else {
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
                    .filter_map(|other| scene.back_edge_index(other)),
            )
            .collect::<Vec<_>>();
        let step = match side {
            Side::Left => -LANE,
            Side::Right => LANE,
        };
        let boundary = scene
            .topology
            .loop_boundaries
            .iter()
            .find(|boundary| boundary.header == scene.topology.loops[index].header)
            .expect("a repeating cycle has a boundary");
        let (boundary_header, boundary_end) = (boundary.header, boundary.end);
        let obstructions = |scene: &Scene| {
            let mut bounds = scene
                .labels
                .iter()
                .map(label::label_rect)
                .collect::<Vec<_>>();
            bounds.extend(
                scene
                    .topology
                    .loop_boundaries
                    .iter()
                    .zip(loop_block::regions(scene))
                    .filter(|(nested, _)| {
                        (boundary_header + 1..boundary_end).contains(&nested.header)
                    })
                    .map(|(_, region)| {
                        (
                            region.left - LANE,
                            region.top - LANE,
                            region.right + LANE,
                            region.bottom + LANE,
                        )
                    }),
            );
            bounds
        };
        let struck = |scene: &Scene| {
            obstructions(scene)
                .into_iter()
                .filter(|rect| route::crosses(&scene.connections[back].points, *rect))
                .map(|rect| match side {
                    Side::Left => rect.0,
                    Side::Right => rect.2,
                })
                .reduce(|outer, edge| match side {
                    Side::Left => outer.min(edge),
                    Side::Right => outer.max(edge),
                })
        };
        // A label or nested boundary can span several lanes. Past the
        // outermost obstruction the climb is clear; a collision left there is
        // on a horizontal run, which stepping farther out cannot clear.
        let rail = scene.connections[back].points[1].x;
        let outermost = obstructions(scene)
            .into_iter()
            .fold(rail, |outer, (left, _, right, _)| match side {
                Side::Left => outer.min(left),
                Side::Right => outer.max(right),
            });
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

/// How far past its body's edge each back edge rail can reach, on the left and
/// on the right.
///
/// A rail sits `(lane + 1)` lanes past the body it climbs, and a lane past any
/// rail already drawn beside a body nested in that one — so a reach grows
/// along the deepest chain of nested cycles rather than with their number.
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
    layout_spaced(model, start, parameters, return_type, true)
        .or_else(|_| layout_spaced(model, start, parameters, return_type, false))
}

fn layout_spaced(
    model: &SemanticModel,
    start: &str,
    parameters: &[String],
    return_type: &str,
    narrow: bool,
) -> Result<Scene, String> {
    // A label is wrapped to fit the standard gap beside its column, and a
    // back edge climbs in that same gap. Where the two want it at once — a label
    // reaching rightward from one column past the rail of the cycle in the
    // next — no rail position clears it, because stepping out goes further
    // into the label and stepping in goes into the body. The room has to come
    // from the columns, and how much is only known once the labels are placed,
    // so the layout is taken again with the gap the last one asked for.
    //
    // That ends, and not by counting attempts. A label is wrapped to a fixed
    // width and a node measured to a fixed one, neither of which the slack
    // changes; the slack widens the gap between two columns and nothing else.
    // Once the gap holds the widest label a connection can hang beside the
    // deepest a rail reaches past a body, the two cannot want the same pixel,
    // so a pass asking for more room past that bound is a renderer defect
    // rather than a topology the columns cannot hold. Every pass adds at least
    // one lane, so the bound is reached in finitely many.
    let (left, right) = contour_reach(model);
    let bound = label::LABEL_WIDTH + left + right + LANE;
    let mut slack = 0;
    loop {
        match attempt(model, start, parameters, return_type, slack, narrow) {
            Ok(scene) => return Ok(scene),
            Err(Blocked::Refused(reason)) => return Err(reason),
            Err(Blocked::Narrow(_)) if slack >= bound => {
                return Err(
                    "an iteration back edge and a connection label want the same gap".to_owned(),
                );
            }
            // Never past the bound: a pass that asks for more than the gap can
            // ever need is answered by a pass at the bound itself, so the error
            // above means the widest gap was tried and still did not hold.
            Err(Blocked::Narrow(wanted)) => slack = (slack + wanted.max(LANE)).min(bound),
        }
    }
}

/// Why one pass of the layout produced no scene.
enum Blocked {
    /// This pass breaks a geometry rule other than the gap conflict below.
    Refused(String),
    /// A back edge could not be drawn clear of a label; the columns need this
    /// much more room between them.
    Narrow(i32),
}

/// Whether the pixels still say what the arrangement decided.
///
/// The geometry checks hold a scene to RFC 0002 §8; this holds it to the
/// witness it is supposed to realize, which is a different question. A
/// transformation can leave crossing-free geometry that draws another
/// structure — two nodes swapped between columns cross nothing — and only this
/// sees that.
///
/// Every node and junction stays on its measured row and column centre,
/// and incident routes meet at the recorded ports.
/// Back edges retain their column boundary after indentation shifts the scene.
pub(super) fn correspondence(scene: &Scene) -> Option<String> {
    let rows = scene.rows();
    let origin =
        scene.node(NodeId::Start).x - scene.column_x(scene.column(Vertex::Node(NodeId::Start)));
    let column_x = |vertex| origin + scene.column_x(scene.column(vertex));
    for node in &scene.nodes {
        let vertex = Vertex::Node(node.id);
        if node.y != rows.line_y(RunLine::Rank(scene.rank(vertex))) {
            return Some(format!("{:?} is drawn away from its row centre", node.id));
        }
        if node.x != column_x(vertex) {
            return Some(format!(
                "{:?} is drawn away from its column centre",
                node.id
            ));
        }
    }
    let mut junctions = BTreeMap::new();
    for connection in &scene.connections {
        let (Some(first), Some(last)) = (connection.points.first(), connection.points.last())
        else {
            return Some("a connection has no route".to_owned());
        };
        if let Source::Exit(exit) = connection.source
            && *first != route::exit_anchor(scene, exit, connection.destination)
        {
            return Some(format!("a connection leaves {exit:?} away from its port"));
        }
        if let Vertex::Node(node) = connection.destination
            && *last != scene.top_anchor(node)
        {
            return Some(format!("a connection reaches {node:?} away from its port"));
        }
        for (vertex, point) in [
            (Vertex::from(connection.source), *first),
            (connection.destination, *last),
        ] {
            if let Vertex::Junction(junction) = vertex {
                if point.x != column_x(vertex) {
                    return Some(format!(
                        "junction {junction} is drawn away from its column centre"
                    ));
                }
                if point.y != rows.line_y(RunLine::Rank(scene.rank(vertex))) {
                    return Some(format!(
                        "junction {junction} is drawn away from its row centre"
                    ));
                }
                if let Some(previous) = junctions.insert(junction, point)
                    && previous != point
                {
                    return Some(format!("the routes of junction {junction} do not meet"));
                }
            }
        }
    }
    back_edge_correspondence(scene)
}

fn back_edge_correspondence(scene: &Scene) -> Option<String> {
    let origin =
        scene.node(NodeId::Start).x - scene.column_x(scene.column(Vertex::Node(NodeId::Start)));
    for (index, contour) in scene.arrangement.contours.iter().enumerate() {
        let tail = scene.topology.loops[index].tail;
        let Some(edge) = scene.back_edge(index) else {
            return Some(format!(
                "the iteration back edge of junction {tail} is missing"
            ));
        };
        if let Some(recorded) = scene.arrangement.back_routes.get(&index) {
            let mut expected = std::iter::once(recorded.arrival)
                .chain(
                    recorded
                        .runs
                        .iter()
                        .rev()
                        .flat_map(|run| [run.exit, run.enter]),
                )
                .chain(std::iter::once(recorded.departure))
                .map(|column| origin + route::back_edge_x(scene, index, column))
                .collect::<Vec<_>>();
            expected.dedup();
            let mut actual = edge
                .points
                .iter()
                .skip(1)
                .take(edge.points.len().saturating_sub(2))
                .map(|point| point.x)
                .collect::<Vec<_>>();
            actual.dedup();
            if actual != expected {
                return Some(format!(
                    "the iteration back edge of junction {tail} changed its recorded corridor"
                ));
            }
            continue;
        }
        let Some(climb) = edge.points.windows(2).find(|pair| pair[1].y < pair[0].y) else {
            return Some(format!(
                "the iteration back edge of junction {tail} has no climb"
            ));
        };
        let anchor = route::contour_anchor(scene, index);
        let offset = (contour.lane as i32 + 1) * LANE;
        let kept = match contour.side {
            Side::Left => climb[0].x <= anchor - offset,
            Side::Right => climb[0].x >= anchor + offset,
        };
        if !kept {
            return Some(format!(
                "the iteration back edge of junction {tail} moved inside its recorded contour"
            ));
        }
    }
    None
}

fn attempt(
    model: &SemanticModel,
    start: &str,
    parameters: &[String],
    return_type: &str,
    slack: i32,
    narrow: bool,
) -> Result<Scene, Blocked> {
    let mut scene = Scene {
        slack,
        narrow,
        width: 0,
        height: 0,
        nodes: Vec::new(),
        parameters: parameter_panel(parameters),
        connections: Vec::new(),
        labels: Vec::new(),
        loop_regions: Vec::new(),
        captions: captions::derive(model, start, return_type),
        reach: contour_reach(model),
        bodies: model
            .topology
            .loops
            .iter()
            .map(|loop_| model.body_vertices(loop_.header))
            .collect(),
        region_bodies: model
            .topology
            .loop_boundaries
            .iter()
            .map(|boundary| model.body_vertices(boundary.header))
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
        .extend(route::back_edges(&scene, model, &rows));
    // The final word on RFC 0002 §8. The arrangement is checked in abstract
    // ranks and columns; this reads the emitted geometry, so it catches
    // anything the abstract check does not model. A disagreement means the
    // realization is wrong, not that another arrangement should be tried.
    if let Some(reason) = route::verify(&scene).or_else(|| correspondence(&scene)) {
        return Err(Blocked::Refused(reason));
    }

    finish(scene)
}

/// Places the labels, settles the coordinates and the canvas, and checks the
/// complete scene.
fn finish(mut scene: Scene) -> Result<Scene, Blocked> {
    scene.labels = label::place_labels(&scene);
    let wanted = clear_labels(&mut scene);
    if wanted > 0 {
        return Err(Blocked::Narrow(wanted));
    }
    scene.loop_regions = loop_block::regions(&scene);
    scene.indent();
    scene.fit();
    // The complete result after every transformation, including the
    // coordinates and canvas that `indent` and `fit` settle, against both the
    // spatial rules and the arrangement it realizes.
    if let Some(reason) = route::verify(&scene)
        .or_else(|| label::verify(&scene))
        .or_else(|| loop_block::verify(&scene))
        .or_else(|| correspondence(&scene))
    {
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

/// Top edge and height of every row, in row order. A visible merge reserves its
/// marker; structural junctions carry no box.
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
    /// zero when the row contains no vertex.
    capture_space: Vec<i32>,
}

impl Rows {
    pub(super) fn line_y(&self, line: RunLine) -> i32 {
        match line {
            RunLine::Rank(rank) => self.top[rank] + self.height[rank] / 2,
            RunLine::Lane { gap, lane } => self.lane_y(gap, lane, self.lanes_in(gap)),
        }
    }

    pub(super) fn lanes_in(&self, gap: usize) -> usize {
        self.lanes.get(gap).copied().unwrap_or(0)
    }

    /// Packs horizontal runs toward the following row, leaving the spare room
    /// below the producers rather than pressing the merge against their exits.
    pub(super) fn lane_y(&self, gap: usize, lane: usize, lanes: usize) -> i32 {
        self.top[gap + 1] - self.capture_space[gap + 1] - (lanes - lane - 1) as i32 * LANE
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
        for (junction, node) in self.topology.junctions.iter().enumerate() {
            let row = self.rank(Vertex::Junction(junction));
            capture_space[row] = gap;
            if !node.merges.is_empty() {
                height[row] = height[row].max(2 * MERGE_RADIUS);
            }
        }
        if let Some(parameters) = &self.parameters {
            let row = self.rank(Vertex::Node(NodeId::Start));
            height[row] = height[row].max(parameters.height);
        }
        let lanes = self.arrangement.gap_lanes.clone();
        let bottom_padding = loop_block::bottom_padding(self);
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
            next += own + (gap + bottom_padding[row]).max(routing);
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
    /// Node and exit columns retain their measured half-width on either side,
    /// including the space for a question's branch description. A column used
    /// only by routing needs one lane instead. This map stays strictly
    /// increasing; if the smaller gaps fail any check, `layout` retries the
    /// same witness with uniform spacing. Bent back edges need that uniform map
    /// to preserve the order of their offsets from both sides of each column.
    pub(super) fn column_x(&self, column: i32) -> i32 {
        let width = self.column_width();
        if !self.narrow || !self.arrangement.back_routes.is_empty() {
            return MARGIN + NODE_WIDTH / 2 + column * width;
        }
        let half = |at| {
            if self
                .topology
                .nodes
                .iter()
                .any(|node| self.column(Vertex::Node(node.id)) == at)
                || self
                    .arrangement
                    .exit_offset
                    .iter()
                    .any(|(exit, offset)| self.column(Vertex::Node(exit.node)) + offset == at)
            {
                width / 2
            } else {
                LANE.midpoint(self.slack)
            }
        };
        let distance: i32 = (0.min(column)..0.max(column))
            .map(|at| half(at) + half(at + 1))
            .sum();
        MARGIN + NODE_WIDTH / 2 + column.signum() * distance
    }

    /// How far apart two columns stand.
    ///
    /// Wide enough that the deepest rail reaching into a gap from the left and
    /// the deepest reaching into it from the right still leave a lane between
    /// them. `contour_reach` measures both, so a rail pushed out by a chain of
    /// nested back edges is held as well as one pushed out by its own lane.
    ///
    /// ponytail: one width for the whole diagram, so one deep contour widens
    /// every gap; give each gap its own width if a diagram looks stretched.
    fn column_width(&self) -> i32 {
        let (left, right) = self.reach;
        let labels = if self.arrangement.back_routes.is_empty() {
            0
        } else {
            2 * label::LABEL_WIDTH
        };
        COLUMN_WIDTH.max(NODE_WIDTH + left + right + LANE + labels) + self.slack
    }

    pub(super) fn column(&self, vertex: Vertex) -> i32 {
        self.arrangement.column[&vertex]
    }

    /// Centres every node on its row, once the rows have their heights.
    fn lift(&mut self, rows: &Rows) {
        for node in &mut self.nodes {
            let row = self.arrangement.rank[&Vertex::Node(node.id)];
            node.y = rows.line_y(RunLine::Rank(row));
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

    /// Where one loop's iteration back edge sits in `connections`, once drawn.
    pub(super) fn back_edge_index(&self, index: usize) -> Option<usize> {
        let tail = self.topology.loops[index].tail;
        self.connections
            .iter()
            .position(|edge| edge.source == Source::Junction(tail))
    }

    pub(super) fn back_edge(&self, index: usize) -> Option<&Connection> {
        self.back_edge_index(index).map(|at| &self.connections[at])
    }

    /// Where one junction is drawn: the end every incident route shares.
    pub(crate) fn junction_at(&self, junction: usize) -> Option<Point> {
        self.connections.iter().find_map(|edge| {
            if edge.source == Source::Junction(junction) {
                edge.points.first().copied()
            } else if edge.destination == Destination::Junction(junction) {
                edge.points.last().copied()
            } else {
                None
            }
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

    /// The default boundary anchor for one exit. Select-to-case connections
    /// override it according to their destination branch.
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
            .min(
                self.loop_regions
                    .iter()
                    .map(|region| region.left)
                    .min()
                    .unwrap_or(MARGIN),
            )
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
        for region in &mut self.loop_regions {
            region.left += shift;
            region.right += shift;
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
        for region in &self.loop_regions {
            right = right.max(region.right);
            bottom = bottom.max(region.bottom);
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
        NodeKind::Call => call::dimensions(label),
        NodeKind::Loop => loop_block::dimensions(label),
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
