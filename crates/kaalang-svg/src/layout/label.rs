//! Places the wire labels: how much room a pair of them needs, how wide and
//! tall one is once wrapped, and where each is drawn.
//!
//! RFC 0002 §6 gives a hand-over to the exit that provides it and a capture to
//! the node that receives it, so each is drawn once however many connections
//! leave or arrive. The two ends of one connection share a single label only
//! when that connection is alone at both of them and the displayed lists match.
//! Identical alternative hand-overs share a label at their merge; an identical
//! sole consumer may share that label as well.

use std::collections::BTreeSet;

use kaalang_model::topology::{Destination, ExitId, Source, Vertex};

use super::{
    BRANCH_LABEL_FONT, COLUMN_WIDTH, CONNECTION_LABEL_FONT, CONNECTION_LABEL_HALO, Connection,
    Label, LabelKind, MIN_VERTICAL_GAP, NODE_WIDTH, Point, Scene,
    text::{text_width, wrap_text},
};

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
/// Lifts a baseline so a label's ink straddles the point it marks.
const BASELINE: i32 = 5;

/// Where a label's lines stack against the point it marks.
#[derive(Clone, Copy)]
enum Stack {
    /// Centred on it, for a label in the middle of a run.
    Around,
    /// Ending at it, so a wrapped label climbs away from the node below.
    Above,
    /// Starting at it, so a wrapped label hangs away from the node above.
    Below,
}

pub(super) fn place_labels(scene: &Scene) -> Vec<Label> {
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
        let placed = scene
            .connections
            .iter()
            .find(|placed| {
                placed.source == connection.source && placed.destination == connection.destination
            })
            .expect("every projected connection is routed");
        let (at, stack, clear) = centre_of(placed);
        labels.extend(wire_label(
            captions.capture_label(node_of(connection.destination)),
            at,
            stack,
            clear,
        ));
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
        let anchor = scene.top_anchor(node.id);
        labels.extend(wire_label(
            captions.capture_label(node.id),
            Point {
                x: anchor.x,
                y: anchor.y - RISE,
            },
            Stack::Above,
            anchor.x,
        ));
    }

    labels
}

/// Identical alternative hand-overs share one label beside their merge, and an
/// identical sole consumer shares it too. Records which exits and which capture
/// the shared label already stands for.
fn place_merge_label(
    labels: &mut Vec<Label>,
    scene: &Scene,
    junction: usize,
    merged_exits: &mut BTreeSet<ExitId>,
    merged_captures: &mut BTreeSet<kaalang_model::topology::NodeId>,
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

    let anchor = merge_anchor(scene, junction);
    labels.extend(wire_label(
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
            description,
            Point { x: anchor.x, y },
            stack,
            anchor.x,
        ));
    } else if !skip_handover {
        labels.extend(wire_label(
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

fn node_of(destination: Destination) -> kaalang_model::topology::NodeId {
    match destination {
        Destination::Node(node) => node,
        Destination::Junction(_) => unreachable!("a shared label needs a node at both ends"),
    }
}

/// Incoming routes end at the visible merge; its outgoing segment owns the
/// vertical below it.
fn merge_anchor(scene: &Scene, junction: usize) -> Point {
    scene
        .connections
        .iter()
        .find(|wire| wire.destination == Destination::Junction(junction))
        .and_then(|wire| wire.points.last().copied())
        .expect("a merge has incoming routes")
}

/// The middle of the run a connection leaves on, beside the wire rather than
/// over it, for the label both of its ends agree on.
fn centre_of(connection: &Connection) -> (Point, Stack, i32) {
    let [start, next, ..] = connection.points[..] else {
        unreachable!("a routed connection has at least two points")
    };
    if start.y == next.y {
        return (
            Point {
                x: i32::midpoint(start.x, next.x),
                y: start.y - RISE,
            },
            Stack::Above,
            start.x,
        );
    }

    (
        Point {
            x: start.x,
            y: i32::midpoint(start.y, next.y) - BASELINE,
        },
        Stack::Around,
        start.x,
    )
}

/// Wraps the wire names one label draws, or nothing when it names none.
/// Wrapping an empty label would yield one blank line, so every caller needs
/// the same guard.
fn wrap_wires(names: &[String]) -> Option<Vec<String>> {
    let names = names
        .iter()
        .filter(|name| !matches!(name.as_str(), "end" | "mut end"))
        .cloned()
        .collect::<Vec<_>>();
    (!names.is_empty()).then(|| wrap_text(&names.join(", "), LABEL_WIDTH, CONNECTION_LABEL_FONT))
}

/// Wraps one label and stacks its lines against `at`. Its left edge, including
/// the halo, stays clear of the adjacent vertical run or node boundary.
fn wire_label(names: &[String], at: Point, stack: Stack, clear: i32) -> Option<Label> {
    let lines = wrap_wires(names)?;
    Some(place_label(lines, LabelKind::Wire, at, stack, clear))
}

fn branch_label(description: &str, at: Point, stack: Stack, clear: i32) -> Label {
    let lines = wrap_text(description, LABEL_WIDTH, LabelKind::Branch.font_size());
    place_label(lines, LabelKind::Branch, at, stack, clear)
}

fn place_label(lines: Vec<String>, kind: LabelKind, at: Point, stack: Stack, clear: i32) -> Label {
    let below_first = (lines.len() as i32 - 1) * kind.line_height();
    let x = (at.x - label_width(&lines, kind.font_size()) / 2)
        .max(clear + CONNECTION_LABEL_HALO + CLEARANCE);

    Label {
        kind,
        at: Point {
            x,
            y: at.y
                - match stack {
                    Stack::Around => below_first / 2,
                    Stack::Above => below_first,
                    Stack::Below => 0,
                },
        },
        lines,
    }
}

/// Leaves enough room in every row gap for a hand-over below one node and a
/// capture above the next.
pub(super) fn vertical_gap(scene: &Scene) -> i32 {
    // ponytail: one global gap keeps routing simple; reserve per-row gaps if
    // tall diagrams become a practical problem.
    let captions = &scene.captions;
    let gap = scene
        .topology
        .exits
        .iter()
        .map(|exit| match captions.branch_description(exit.id) {
            Some(description) => {
                let lines = wrap_text(description, LABEL_WIDTH, LabelKind::Branch.font_size());
                vertical_label_gap(lines.len(), LabelKind::Branch)
            }
            None => vertical_label_gap(
                label_line_count(captions.handover(exit.id)),
                LabelKind::Wire,
            ),
        })
        .chain(scene.topology.nodes.iter().map(|node| {
            vertical_label_gap(
                label_line_count(captions.capture_label(node.id)),
                LabelKind::Wire,
            )
        }))
        .max()
        .unwrap_or(0);
    MIN_VERTICAL_GAP.max(gap)
}

fn vertical_label_gap(lines: usize, kind: LabelKind) -> i32 {
    if lines == 0 {
        return 0;
    }
    let drop = match kind {
        LabelKind::Wire => DROP,
        LabelKind::Branch => BRANCH_DROP,
    };
    drop + RISE
        + kind.font_size()
        + 2 * CONNECTION_LABEL_HALO
        + 2 * (lines as i32 - 1) * kind.line_height()
}

fn label_line_count(names: &[String]) -> usize {
    wrap_wires(names).map_or(0, |lines| lines.len())
}

/// The rectangle a label's ink and halo occupy, matching how the serializer
/// places it: starting at `at.x`, its first baseline at `at.y`. Left, top,
/// right, bottom, like `Scene::bounds`.
pub(super) fn label_rect(label: &Label) -> (i32, i32, i32, i32) {
    let font_size = label.kind.font_size();
    let width = label_width(&label.lines, font_size);
    let last_baseline = label.at.y + (label.lines.len() as i32 - 1) * label.kind.line_height();

    (
        label.at.x - CONNECTION_LABEL_HALO,
        label.at.y - font_size - CONNECTION_LABEL_HALO,
        label.at.x + width + CONNECTION_LABEL_HALO,
        last_baseline + font_size / 2 + CONNECTION_LABEL_HALO,
    )
}

/// Reports the first label that leaves the canvas or reaches into a node.
///
/// RFC 0002 §8 keeps every standalone label inside the diagram and clear of the
/// nodes. `route::verify` holds the connections to their half of that rule;
/// this holds the labels to theirs, on the same emitted geometry.
///
/// A transformation is gated on `clearance` instead, which is the half no later
/// step repairs. The other two are repaired: a return crossing a label is what
/// `clear_labels` steps the rail out of, and a label left of the origin is what
/// `indent` slides the drawing over for. Rejecting a compaction for either
/// would refuse geometry that is about to be put right.
pub(super) fn verify(scene: &Scene) -> Option<String> {
    clearance(scene)
        .or_else(|| {
            scene.labels.iter().find_map(|label| {
                let (left, top, right, bottom) = label_rect(label);
                (left < 0 || top < 0 || right > scene.width || bottom > scene.height)
                    .then(|| format!("the label `{}` leaves the canvas", label.lines.join(" ")))
            })
        })
        .or_else(|| {
            scene.labels.iter().find_map(|label| {
                let rect = label_rect(label);
                // A return climbs beside a body it was placed clear of, not clear
                // of the labels that body hangs. Nothing else compares the two:
                // the climb is not a route with a label of its own, so the
                // crossing check in `route` never brings them together.
                scene
                    .connections
                    .iter()
                    .filter(|edge| scene.is_back_edge(edge))
                    .flat_map(|edge| edge.points.windows(2))
                    .any(|segment| super::route::enters(segment[0], segment[1], rect))
                    .then(|| {
                        format!(
                            "a loop return crosses the label `{}`",
                            label.lines.join(" ")
                        )
                    })
            })
        })
}

/// The half of the label contract no later step repairs: a label stays clear of
/// every node and of the parameter panel.
///
/// Nothing moves a label off a node afterwards — `clear_labels` steps rails,
/// not labels, and `indent` and `fit` move everything together — so this is
/// what a transformation has to be held to, and the only half of the contract
/// that means anything before the coordinates are settled.
pub(super) fn clearance(scene: &Scene) -> Option<String> {
    for label in &scene.labels {
        let rect = label_rect(label);
        let names = label.lines.join(" ");
        let nodes = scene.nodes.iter().map(|node| {
            (
                Scene::bounds(node),
                format!("the node `{}`", node.lines.join(" ")),
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
const fn overlaps(rect: (i32, i32, i32, i32), other: (i32, i32, i32, i32)) -> bool {
    rect.2 > other.0 && rect.0 < other.2 && rect.3 > other.1 && rect.1 < other.3
}

fn label_width(lines: &[String], font_size: i32) -> i32 {
    lines
        .iter()
        .map(|line| text_width(line, font_size))
        .max()
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::captions::Captions;
    use kaalang_model::topology::{NodeId, Topology};

    /// One label of a known width, so a test can put it where it must not be.
    fn scene(at: Point, node: Option<(i32, i32)>) -> Scene {
        Scene {
            narrow: false,
            reach: (0, 0),
            slack: 0,
            bodies: Vec::new(),
            width: 200,
            height: 200,
            topology: Topology::default(),
            arrangement: kaalang_model::Arrangement::default(),
            captions: Captions::default(),
            nodes: node
                .into_iter()
                .map(|(x, y)| super::super::Node {
                    id: NodeId::Block(0),
                    x,
                    y,
                    width: 40,
                    height: 20,
                    lines: vec!["Do the work.".to_owned()],
                })
                .collect(),
            parameters: None,
            connections: vec![],
            labels: vec![Label {
                kind: LabelKind::Wire,
                lines: vec!["end".to_owned()],
                at,
            }],
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
}
