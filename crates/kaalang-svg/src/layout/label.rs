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

use unicode_segmentation::UnicodeSegmentation;

use crate::topology::{Destination, Exit, Source, Topology, Vertex};

use super::{
    BRANCH_LABEL_FONT, COLUMN_WIDTH, CONNECTION_LABEL_FONT, CONNECTION_LABEL_HALO, Connection,
    Label, LabelKind, MIN_VERTICAL_GAP, NODE_WIDTH, Point, Scene,
    text::{text_width, wrap_text},
};

/// A label beside a vertical run must fit before the next column's node,
/// including clearance and its halo on both sides.
const LABEL_WIDTH: i32 = COLUMN_WIDTH - NODE_WIDTH / 2 - 2 * (CONNECTION_LABEL_HALO + CLEARANCE);
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
    let shared = topology
        .connections
        .iter()
        .filter(|connection| topology.shares_label(connection))
        .collect::<Vec<_>>();
    let mut labels = Vec::new();
    let mut merged_exits = BTreeSet::new();
    let mut merged_captures = BTreeSet::new();

    for (junction, merge) in topology.junctions.iter().enumerate() {
        if merge.wires.is_empty() {
            continue;
        }
        let Some(exits) = topology
            .incoming(Vertex::Junction(junction))
            .map(|connection| match connection.source {
                Source::Exit(exit)
                    if topology.handover(exit) == merge.wires
                        && topology.leaving(exit).count() == 1 =>
                {
                    Some(exit)
                }
                _ => None,
            })
            .collect::<Option<Vec<_>>>()
        else {
            continue;
        };
        merged_exits.extend(exits);

        let mut outgoing = topology.outgoing(Vertex::Junction(junction));
        if let Some(Destination::Node(node)) = outgoing.next().map(|wire| wire.destination)
            && outgoing.next().is_none()
            && topology.single_arrival(node)
            && topology.capture(node) == merge.wires
        {
            merged_captures.insert(node);
        }

        let anchor = merge_anchor(scene, junction);
        labels.extend(wire_label(
            &merge.wires,
            Point {
                x: anchor.x,
                y: anchor.y + DROP,
            },
            Stack::Below,
            anchor.x,
        ));
    }

    for connection in &shared {
        if let Source::Exit(exit) = connection.source
            && topology.exit(exit).branch_description.is_some()
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
            &topology.capture_label(node_of(connection.destination)),
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
            || (shared_handover && exit.branch_description.is_none());
        place_exit_labels(&mut labels, exit, scene.exit_anchor(exit.id), skip_handover);
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
            &topology.capture_label(node.id),
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

fn place_exit_labels(labels: &mut Vec<Label>, exit: &Exit, anchor: Point, skip_handover: bool) {
    if let Some(description) = &exit.branch_description {
        let (y, stack) = if exit.id.branch == Some(0) {
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
            &exit.handover,
            Point {
                x: anchor.x,
                y: anchor.y + DROP,
            },
            Stack::Below,
            anchor.x,
        ));
    }
}

fn node_of(destination: Destination) -> crate::topology::NodeId {
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
pub(super) fn vertical_gap(topology: &Topology) -> i32 {
    // ponytail: one global gap keeps routing simple; reserve per-row gaps if
    // tall diagrams become a practical problem.
    let gap = topology
        .exits
        .iter()
        .map(|exit| match exit.branch_description.as_deref() {
            Some(description) => {
                vertical_label_gap(branch_label_line_count(description), LabelKind::Branch)
            }
            None => vertical_label_gap(label_line_count(&exit.handover), LabelKind::Wire),
        })
        .chain(topology.nodes.iter().map(|node| {
            vertical_label_gap(
                label_line_count(&topology.capture_label(node.id)),
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

fn branch_label_line_count(description: &str) -> usize {
    wrap_text(description, LABEL_WIDTH, LabelKind::Branch.font_size()).len()
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
pub(super) fn verify(scene: &Scene) -> Option<String> {
    for label in &scene.labels {
        let (left, top, right, bottom) = label_rect(label);
        let names = label.lines.join(" ");
        if left < 0 || top < 0 || right > scene.width || bottom > scene.height {
            return Some(format!("the label `{names}` leaves the canvas"));
        }
        for node in &scene.nodes {
            let (node_left, node_top, node_right, node_bottom) = Scene::bounds(node);
            if right > node_left && left < node_right && bottom > node_top && top < node_bottom {
                return Some(format!(
                    "the label `{names}` reaches into the node `{}`",
                    node.lines.join(" ")
                ));
            }
        }
        if let Some(parameters) = &scene.parameters {
            let (panel_left, panel_top, panel_right, panel_bottom) =
                Scene::parameter_bounds(parameters);
            if right > panel_left && left < panel_right && bottom > panel_top && top < panel_bottom
            {
                return Some(format!(
                    "the label `{names}` reaches into the parameter panel"
                ));
            }
        }
    }

    None
}

fn label_width(lines: &[String], font_size: i32) -> i32 {
    lines
        .iter()
        .map(|line| text_width(&line.graphemes(true).collect::<Vec<_>>(), font_size))
        .max()
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::topology::NodeId;

    /// One label of a known width, so a test can put it where it must not be.
    fn scene(at: Point, node: Option<(i32, i32)>) -> Scene {
        Scene {
            width: 200,
            height: 200,
            topology: Topology {
                order: vec![],
                back_edges: vec![],
                loops: vec![],
                nodes: vec![],
                exits: vec![],
                junctions: vec![],
                connections: vec![],
                vertices: vec![],
            },
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
