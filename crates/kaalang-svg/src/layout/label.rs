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

use crate::topology::{Destination, Source, Topology, Vertex};

use super::{
    COLUMN_WIDTH, CONNECTION_LABEL_FONT, CONNECTION_LABEL_HALO, CONNECTION_LINE_HEIGHT, Connection,
    Label, MIN_VERTICAL_GAP, NODE_WIDTH, Point, Scene,
    text::{text_width, wrap_text},
};

/// A label beside a vertical run must fit before the next column's node,
/// including clearance and its halo on both sides.
const LABEL_WIDTH: i32 = COLUMN_WIDTH - NODE_WIDTH / 2 - 2 * (CONNECTION_LABEL_HALO + CLEARANCE);
/// Sideways offset of a label from the run it names.
const ASIDE: i32 = 42;
/// Clear space between a label's halo and the vertical run or node beside it.
const CLEARANCE: i32 = 8;
/// Rise of a label above a horizontal run or a node's top border. Enough for
/// the halo to clear the border too, so a capture never paints over the node it
/// belongs to.
const RISE: i32 = CONNECTION_LABEL_FONT / 2 + CONNECTION_LABEL_HALO;
/// Drop of a hand-over label below the exit it leaves by.
const DROP: i32 = 18;
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
            && topology.arriving(node).count() == 1
            && topology.capture(node) == merge.wires
        {
            merged_captures.insert(node);
        }

        let anchor = merge_anchor(scene, junction);
        labels.extend(wire_label(
            &merge.wires,
            Point {
                x: anchor.x + ASIDE,
                y: anchor.y + DROP,
            },
            Stack::Below,
            anchor.x,
        ));
    }

    for connection in &shared {
        let (at, stack, clear) = centre_of(scene.connections.iter().find(|placed| {
            placed.source == connection.source && placed.destination == connection.destination
        }));
        labels.extend(wire_label(
            &topology.capture_label(node_of(connection.destination)),
            at,
            stack,
            clear,
        ));
    }

    for exit in &topology.exits {
        if merged_exits.contains(&exit.id)
            || shared
                .iter()
                .any(|connection| connection.source == Source::Exit(exit.id))
        {
            continue;
        }
        let anchor = scene.exit_anchor(exit.id);
        labels.extend(wire_label(
            &exit.handover,
            Point {
                x: anchor.x + ASIDE,
                y: anchor.y + DROP,
            },
            Stack::Below,
            anchor.x,
        ));
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
                x: anchor.x + ASIDE,
                y: anchor.y - RISE,
            },
            Stack::Above,
            anchor.x,
        ));
    }

    labels
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
fn centre_of(connection: Option<&Connection>) -> (Point, Stack, i32) {
    let Some([start, next]) = connection.map(|connection| {
        let [start, next, ..] = connection.points[..] else {
            unreachable!("a routed connection has at least two points")
        };
        [start, next]
    }) else {
        unreachable!("every projected connection is routed")
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
            x: start.x + ASIDE,
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
    (!names.is_empty()).then(|| wrap_text(&names.join(", "), LABEL_WIDTH, CONNECTION_LABEL_FONT))
}

/// Wraps one label and stacks its lines against `at`. Its left edge, including
/// the halo, stays clear of the adjacent vertical run or node boundary.
fn wire_label(names: &[String], at: Point, stack: Stack, clear: i32) -> Option<Label> {
    let lines = wrap_wires(names)?;
    let below_first = (lines.len() as i32 - 1) * CONNECTION_LINE_HEIGHT;
    let x =
        at.x.max(clear + label_width(&lines) / 2 + CONNECTION_LABEL_HALO + CLEARANCE);

    Some(Label {
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
    })
}

/// Leaves enough room in every row gap for a hand-over below one node and a
/// capture above the next.
pub(super) fn vertical_gap(topology: &Topology) -> i32 {
    // ponytail: one global gap keeps routing simple; reserve per-row gaps if
    // tall diagrams become a practical problem.
    let lines = topology
        .exits
        .iter()
        .map(|exit| label_line_count(&exit.handover))
        .chain(
            topology
                .nodes
                .iter()
                .map(|node| label_line_count(&topology.capture_label(node.id))),
        )
        .max()
        .unwrap_or(0) as i32;
    if lines == 0 {
        return MIN_VERTICAL_GAP;
    }

    MIN_VERTICAL_GAP.max(
        DROP + RISE
            + CONNECTION_LABEL_FONT
            + 2 * CONNECTION_LABEL_HALO
            + 2 * (lines - 1) * CONNECTION_LINE_HEIGHT,
    )
}

fn label_line_count(names: &[String]) -> usize {
    wrap_wires(names).map_or(0, |lines| lines.len())
}

/// Space above a node that its capture label and halo occupy. Horizontal
/// arrivals stay above this strip, leaving the label beside their final descent.
pub(super) fn capture_space(names: &[String]) -> i32 {
    let lines = label_line_count(names);
    if lines == 0 {
        return 0;
    }
    RISE + CONNECTION_LABEL_FONT
        + CONNECTION_LABEL_HALO
        + CLEARANCE
        + (lines as i32 - 1) * CONNECTION_LINE_HEIGHT
}

/// Right and bottom extent of a label, matching how the serializer places it:
/// centred on `at.x`, its first baseline at `at.y`.
pub(super) fn label_bounds(label: &Label) -> (i32, i32) {
    let width = label_width(&label.lines);
    let last_baseline = label.at.y + (label.lines.len() as i32 - 1) * CONNECTION_LINE_HEIGHT;

    (
        label.at.x + width / 2 + CONNECTION_LABEL_HALO,
        last_baseline + CONNECTION_LABEL_FONT / 2 + CONNECTION_LABEL_HALO,
    )
}

pub(super) fn label_width(lines: &[String]) -> i32 {
    lines
        .iter()
        .map(|line| {
            text_width(
                &line.graphemes(true).collect::<Vec<_>>(),
                CONNECTION_LABEL_FONT,
            )
        })
        .max()
        .unwrap_or_default()
}
