//! Measures and places wire labels under RFC 0002 §6. Shared connection labels
//! use the capture position; identical alternatives share their merge's label.

use std::collections::BTreeSet;

use kaalang_compiler::{
    RunLine,
    topology::{Destination, ExitId, NodeId, Source, Vertex},
};

use super::{
    BRANCH_LABEL_FONT, COLUMN_WIDTH, CONNECTION_LABEL_FONT, CONNECTION_LABEL_HALO, Label,
    LabelKind, MIN_VERTICAL_GAP, NODE_WIDTH, Point, Rows, Scene,
};
use crate::text::{RichText, block_metrics, joined, line_ink, text_width, wrap_literal, wrap_text};

/// A label beside a vertical run must fit before the next column's node,
/// including clearance and its halo on both sides.
pub(super) const LABEL_WIDTH: i32 =
    COLUMN_WIDTH - NODE_WIDTH / 2 - 2 * (CONNECTION_LABEL_HALO + CLEARANCE);
/// Clear space between a label's halo and the vertical run or node beside it.
const CLEARANCE: i32 = 8;
/// Rise of a label above a horizontal run or a node's top border. Enough for
/// the halo to clear the border too, so a capture never paints over the node it
/// belongs to.
const RISE: i32 = CONNECTION_LABEL_FONT / 2 + CONNECTION_LABEL_HALO;
/// Rise of a larger question-branch description above its horizontal exit.
const BRANCH_RISE: i32 = BRANCH_LABEL_FONT / 2 + CONNECTION_LABEL_HALO;
/// Drop of a hand-over label below the exit it leaves by.
const DROP: i32 = 18;
/// Drop of a larger question-branch description below its exit.
const BRANCH_DROP: i32 = 20;

/// Where a label's lines stack against the point it marks.
#[derive(Clone, Copy)]
enum Stack {
    /// Ending at it, so a wrapped label climbs away from the node below.
    Above,
    /// Starting at it, so a wrapped label hangs away from the node above.
    Below,
}

pub(super) fn place_labels(scene: &Scene, rows: &Rows) -> Vec<Label> {
    let topology = &scene.topology;
    let captions = &scene.captions;
    let shared = topology
        .connections
        .iter()
        .filter(|connection| captions.shares_label(connection))
        .collect::<Vec<_>>();
    let mut labels = Vec::new();
    let mut merged_exits = BTreeSet::new();
    let mut merged_captures = BTreeSet::new();

    for junction in 0..topology.junctions.len() {
        place_merge_label(
            &mut labels,
            scene,
            rows,
            junction,
            &mut merged_exits,
            &mut merged_captures,
        );
    }

    for connection in &shared {
        if let Source::Exit(exit) = connection.source
            && captions.branch_description(exit).is_some()
        {
            continue;
        }
        labels.extend(capture_label(scene, node_of(connection.destination)));
    }

    for exit in &topology.exits {
        let shared_handover = shared
            .iter()
            .any(|connection| connection.source == Source::Exit(exit.id));
        let skip_handover = merged_exits.contains(&exit.id)
            || (shared_handover && captions.branch_description(exit.id).is_none());
        place_exit_labels(
            &mut labels,
            scene,
            exit.id,
            scene.exit_anchor(exit.id),
            skip_handover,
        );
    }

    for node in &topology.nodes {
        if merged_captures.contains(&node.id)
            || shared
                .iter()
                .any(|connection| connection.destination == Destination::Node(node.id))
        {
            continue;
        }
        labels.extend(capture_label(scene, node.id));
    }

    labels
}

/// Captures stay above the receiving node, including labels that also represent
/// the preceding hand-over.
fn capture_label(scene: &Scene, node: NodeId) -> Option<Label> {
    let anchor = scene.top_anchor(node);
    wire_label(
        Vertex::Node(node),
        scene.captions.capture_label(node),
        Point {
            x: anchor.x,
            y: anchor.y - RISE,
        },
        Stack::Above,
        anchor.x,
    )
}

/// Identical alternative hand-overs share one label beside their merge, and an
/// identical sole consumer shares it too. Records which exits and which capture
/// the shared label already stands for.
fn place_merge_label(
    labels: &mut Vec<Label>,
    scene: &Scene,
    rows: &Rows,
    junction: usize,
    merged_exits: &mut BTreeSet<ExitId>,
    merged_captures: &mut BTreeSet<NodeId>,
) {
    let topology = &scene.topology;
    let captions = &scene.captions;
    let wires = captions.junction_wires(junction);
    if wires.is_empty() {
        return;
    }
    let Some(exits) = topology
        .incoming(Vertex::Junction(junction))
        .map(|connection| match connection.source {
            Source::Exit(exit)
                if captions.handover(exit) == wires && topology.leaving(exit).count() == 1 =>
            {
                Some(exit)
            }
            _ => None,
        })
        .collect::<Option<Vec<_>>>()
    else {
        return;
    };
    merged_exits.extend(exits);

    let mut outgoing = topology.outgoing(Vertex::Junction(junction));
    if let Some(Destination::Node(node)) = outgoing.next().map(|wire| wire.destination)
        && outgoing.next().is_none()
        && topology.single_arrival(node)
        && captions.capture(node) == wires
    {
        merged_captures.insert(node);
    }

    let anchor = merge_anchor(scene, rows, junction);
    labels.extend(wire_label(
        Vertex::Junction(junction),
        wires,
        Point {
            x: anchor.x,
            y: anchor.y + DROP,
        },
        Stack::Below,
        anchor.x,
    ));
}

fn place_exit_labels(
    labels: &mut Vec<Label>,
    scene: &Scene,
    exit: ExitId,
    anchor: Point,
    skip_handover: bool,
) {
    if let Some(description) = scene.captions.branch_description(exit) {
        let (y, stack) = if exit.branch == Some(0) {
            (anchor.y + BRANCH_DROP, Stack::Below)
        } else {
            (anchor.y - BRANCH_RISE, Stack::Above)
        };
        labels.push(branch_label(
            Vertex::Node(exit.node),
            description,
            Point { x: anchor.x, y },
            stack,
            anchor.x,
        ));
    } else if !skip_handover {
        labels.extend(wire_label(
            Vertex::Node(exit.node),
            scene.captions.handover(exit),
            Point {
                x: anchor.x,
                y: anchor.y + DROP,
            },
            Stack::Below,
            anchor.x,
        ));
    }
}

fn node_of(destination: Destination) -> NodeId {
    match destination {
        Destination::Node(node) => node,
        Destination::Junction(_) => unreachable!("a shared label needs a node at both ends"),
    }
}

/// Incoming routes end at the visible merge; its outgoing segment owns the
/// vertical below it.
fn merge_anchor(scene: &Scene, rows: &Rows, junction: usize) -> Point {
    let vertex = Vertex::Junction(junction);
    Point {
        x: scene.column_x(scene.column(vertex)),
        y: rows.line_y(RunLine::Rank(scene.rank(vertex))),
    }
}

/// Wraps the wire names one label draws, or nothing when it names none.
/// Wrapping an empty label would yield one blank line, so every caller needs
/// the same guard.
fn wrap_wires(names: &[String]) -> Option<Vec<RichText>> {
    (!names.is_empty()).then(|| wrap_literal(&names.join(", "), LABEL_WIDTH, CONNECTION_LABEL_FONT))
}

/// Wraps one label and stacks its lines against `at`. Its left edge, including
/// the halo, stays clear of the adjacent vertical run or node boundary.
fn wire_label(
    owner: Vertex,
    names: &[String],
    at: Point,
    stack: Stack,
    clear: i32,
) -> Option<Label> {
    let lines = wrap_wires(names)?;
    Some(place_label(owner, lines, LabelKind::Wire, at, stack, clear))
}

fn branch_label(
    owner: Vertex,
    description: &RichText,
    at: Point,
    stack: Stack,
    clear: i32,
) -> Label {
    let lines = wrap_text(description, LABEL_WIDTH, LabelKind::Branch.font_size());
    place_label(owner, lines, LabelKind::Branch, at, stack, clear)
}

fn place_label(
    owner: Vertex,
    lines: Vec<RichText>,
    kind: LabelKind,
    at: Point,
    stack: Stack,
    clear: i32,
) -> Label {
    let x = (at.x - label_width(&lines, kind.font_size()) / 2)
        .max(clear + CONNECTION_LABEL_HALO + CLEARANCE);
    let (baseline_span, above, below) = extent(&lines, kind);

    Label {
        owner,
        kind,
        at: Point {
            x,
            y: at.y
                - match stack {
                    Stack::Above => baseline_span + below,
                    Stack::Below => -above,
                },
        },
        lines,
    }
}

/// Leaves enough room in each row gap for the labels actually drawn there.
pub(super) fn vertical_gaps(scene: &Scene) -> Vec<i32> {
    let mut gaps = vec![MIN_VERTICAL_GAP; scene.arrangement.ranks];
    for label in &scene.labels {
        let row = scene.rank(label.owner);
        let below = match label.owner {
            Vertex::Node(node) => label.at.y >= scene.node(node).y,
            Vertex::Junction(_) => true,
        };
        let Some(gap) = (if below { Some(row) } else { row.checked_sub(1) }) else {
            continue;
        };
        gaps[gap] = gaps[gap].max(vertical_label_gap(&label.lines, label.kind));
    }
    gaps
}

fn vertical_label_gap(lines: &[RichText], kind: LabelKind) -> i32 {
    if lines.is_empty() {
        return 0;
    }
    let drop = match kind {
        LabelKind::Wire => DROP,
        LabelKind::Branch => BRANCH_DROP,
    };
    let (baseline_span, above, below) = extent(lines, kind);
    drop + RISE + kind.font_size() + 2 * CONNECTION_LABEL_HALO + 2 * baseline_span + above + below
}

/// How far a label's last baseline sits below its first, and how far its ink
/// reaches past a plain first line's top and a plain last line's bottom.
fn extent(lines: &[RichText], kind: LabelKind) -> (i32, i32, i32) {
    let font_size = kind.font_size();
    let (Some(first), Some(last)) = (lines.first(), lines.last()) else {
        return (0, 0, 0);
    };
    let baselines = block_metrics(lines, font_size, kind.line_height()).baselines;
    (
        baselines[baselines.len() - 1] - baselines[0],
        (line_ink(first.spans(), font_size).0 - font_size).max(0),
        (line_ink(last.spans(), font_size).1 - font_size / 2).max(0),
    )
}

/// The rectangle a label's ink and halo occupy, matching how the serializer
/// places it: starting at `at.x`, its first baseline at `at.y`. Left, top,
/// right, bottom, like `Scene::bounds`.
pub(super) fn label_rect(label: &Label) -> (i32, i32, i32, i32) {
    let font_size = label.kind.font_size();
    let width = label_width(&label.lines, font_size);
    let metrics = block_metrics(&label.lines, font_size, label.kind.line_height());
    let first = metrics.baselines.first().copied().unwrap_or_default();
    let mut top = label.at.y - font_size;
    let mut bottom = label.at.y + font_size / 2;
    for (index, line) in label.lines.iter().enumerate() {
        let baseline = label.at.y + metrics.baselines[index] - first;
        let (ascent, descent) = line_ink(line.spans(), font_size);
        top = top.min(baseline - ascent.max(font_size));
        bottom = bottom.max(baseline + descent.max(font_size / 2));
    }

    (
        label.at.x - CONNECTION_LABEL_HALO,
        top - CONNECTION_LABEL_HALO,
        label.at.x + width + CONNECTION_LABEL_HALO,
        bottom + CONNECTION_LABEL_HALO,
    )
}

/// Checks final labels against the canvas, nodes, panel, and back edges.
/// Intermediate transformations use `clearance`: later steps can still move
/// rails away from labels and translate the scene inside the canvas.
pub(super) fn verify(scene: &Scene) -> Option<String> {
    clearance(scene)
        .or_else(|| {
            scene.labels.iter().find_map(|label| {
                let (left, top, right, bottom) = label_rect(label);
                (left < 0 || top < 0 || right > scene.width || bottom > scene.height).then(|| {
                    format!(
                        "the label `{}` leaves the canvas",
                        joined(&label.lines, " ")
                    )
                })
            })
        })
        .or_else(|| {
            scene.labels.iter().find_map(|label| {
                let rect = label_rect(label);
                // Route verification does not compare back edges with standalone labels.
                scene
                    .connections
                    .iter()
                    .filter(|edge| scene.is_back_edge(edge))
                    .any(|edge| super::route::crosses(&edge.points, rect))
                    .then(|| {
                        format!(
                            "an iteration back edge crosses the label `{}`",
                            joined(&label.lines, " ")
                        )
                    })
            })
        })
}

/// Checks label clearance from nodes and the parameter panel. Later rail moves
/// and scene translations cannot repair these collisions.
pub(super) fn clearance(scene: &Scene) -> Option<String> {
    for label in &scene.labels {
        let rect = label_rect(label);
        let names = joined(&label.lines, " ");
        let nodes = scene.nodes.iter().map(|node| {
            (
                Scene::bounds(node),
                format!("the node `{}`", joined(&node.lines, " ")),
            )
        });
        let panel = scene.parameters.as_ref().map(|parameters| {
            (
                Scene::parameter_bounds(parameters),
                "the parameter panel".to_owned(),
            )
        });
        if let Some((_, what)) = nodes.chain(panel).find(|(other, _)| overlaps(rect, *other)) {
            return Some(format!("the label `{names}` reaches into {what}"));
        }
    }
    None
}

/// Whether two rectangles share any area, each as left, top, right, bottom.
pub(super) const fn overlaps(rect: (i32, i32, i32, i32), other: (i32, i32, i32, i32)) -> bool {
    rect.2 > other.0 && rect.0 < other.2 && rect.3 > other.1 && rect.1 < other.3
}

/// Whether `outer` holds all of `inner`, edges included.
pub(super) const fn contains(outer: (i32, i32, i32, i32), inner: (i32, i32, i32, i32)) -> bool {
    inner.0 >= outer.0 && inner.1 >= outer.1 && inner.2 <= outer.2 && inner.3 <= outer.3
}

fn label_width(lines: &[RichText], font_size: i32) -> i32 {
    lines
        .iter()
        .map(|line| text_width(line, font_size))
        .max()
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use kaalang_compiler::topology::{NodeId, Topology};

    /// One label of a known width, so a test can put it where it must not be.
    fn scene(at: Point, node: Option<(i32, i32)>) -> Scene {
        Scene {
            narrow: false,
            reach: std::collections::BTreeMap::new(),
            slack: 0,
            bodies: Vec::new(),
            region_bodies: Vec::new(),
            width: 200,
            height: 200,
            topology: Topology::default(),
            arrangement: kaalang_compiler::Arrangement::default(),
            captions: std::rc::Rc::default(),
            nodes: node
                .into_iter()
                .map(|(x, y)| super::super::Node {
                    id: NodeId::Block(0),
                    x,
                    y,
                    width: 40,
                    height: 20,
                    lines: vec![RichText::literal("Do the work.")],
                })
                .collect(),
            parameters: None,
            connections: vec![],
            labels: vec![Label {
                owner: Vertex::Node(NodeId::Block(0)),
                kind: LabelKind::Wire,
                lines: vec![RichText::literal("end")],
                at,
            }],
            loop_regions: Vec::new(),
        }
    }

    #[test]
    fn labels_are_rejected_off_canvas_or_over_a_node() {
        let inside = Point { x: 100, y: 100 };
        assert_eq!(verify(&scene(inside, None)), None);
        // Far enough that the halo clears the node too.
        assert_eq!(verify(&scene(inside, Some((100, 160)))), None);

        for corner in [
            Point { x: 2, y: 100 },
            Point { x: 100, y: 2 },
            Point { x: 198, y: 100 },
            Point { x: 100, y: 198 },
        ] {
            assert_eq!(
                verify(&scene(corner, None)).as_deref(),
                Some("the label `end` leaves the canvas"),
                "{corner:?}"
            );
        }

        assert_eq!(
            verify(&scene(inside, Some((100, 100)))).as_deref(),
            Some("the label `end` reaches into the node `Do the work.`")
        );
    }

    #[test]
    fn tall_math_stacks_away_from_the_connection() {
        let text = RichText::markdown(r"$$\frac{\frac{1}{2}}{\frac{3}{4}}$$");
        for (stack, y) in [(Stack::Above, -BRANCH_RISE), (Stack::Below, BRANCH_DROP)] {
            let label = branch_label(
                Vertex::Node(NodeId::Start),
                &text,
                Point { x: 0, y },
                stack,
                0,
            );
            let (_, top, _, bottom) = label_rect(&label);
            match stack {
                Stack::Above => assert!(bottom <= 0, "bottom = {bottom}"),
                Stack::Below => assert!(top >= 0, "top = {top}"),
            }
        }
    }
}
