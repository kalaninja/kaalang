//! Owns the connection labels: how much room a pair of them needs, how wide
//! and tall one is once wrapped, and where each is placed.

use kaalang_model::{BlockKind, SemanticModel};
use syn::Ident;
use unicode_segmentation::UnicodeSegmentation;

use super::{
    Builder, EDGE_LABEL_FONT, EDGE_LABEL_HALO, EDGE_LINE_HEIGHT, Edge, Label, MIN_VERTICAL_GAP,
    Point, drawn,
    text::{text_width, wrap_text},
};

/// Text budget for an edge label, which floats free of any node.
const EDGE_LABEL_WIDTH: i32 = 240;
/// Sideways offset of a connection label from the wire it names.
const LABEL_ASIDE: i32 = 42;
/// Rise of a label that sits above a horizontal run or a node's top border.
const LABEL_RISE: i32 = 8;
/// Drop of a hand-over label below the exit it leaves by.
const LABEL_DROP: i32 = 18;
/// Lifts a baseline so a label's ink straddles the point it marks.
const LABEL_BASELINE: i32 = 5;

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

impl Builder<'_> {
    /// Places every connection label at the end whose wires it names. A node's
    /// capture is drawn once above it, and a connection that hands over
    /// something else names that below its own exit. A lone connection handing
    /// over exactly what its destination captures needs only the one label,
    /// centred on the run.
    pub(super) fn place_labels(&mut self) {
        let mut labels = Vec::new();
        let mut seen = Vec::new();
        for edge in &self.scene.edges {
            if seen.contains(&edge.to) {
                continue;
            }
            seen.push(edge.to);
            let arrivals = self
                .scene
                .edges
                .iter()
                .filter(|arrival| arrival.to == edge.to)
                .collect::<Vec<_>>();

            if arrivals.len() == 1 && edge.handover == edge.capture {
                let (at, exit) = centre_of(edge);
                labels.extend(wire_label(&edge.capture, at, Stack::Around, exit));
                continue;
            }

            let top = self.top_anchor(edge.to);
            labels.extend(wire_label(
                &edge.capture,
                Point {
                    x: top.x + LABEL_ASIDE,
                    y: top.y - LABEL_RISE,
                },
                Stack::Above,
                None,
            ));
            for arrival in arrivals {
                if arrival.handover != arrival.capture {
                    let (at, exit) = exit_of(arrival, self.node(arrival.from).x);
                    labels.extend(wire_label(&arrival.handover, at, Stack::Below, exit));
                }
            }
        }
        self.scene.labels = labels;
    }
}

/// Wraps the wire names one connection end draws, or nothing when it names
/// none. Wrapping an empty label would yield one blank line, so every caller
/// needs the same guard.
fn wrap_wires(names: &[String]) -> Option<Vec<String>> {
    (!names.is_empty()).then(|| wrap_text(&names.join(", "), EDGE_LABEL_WIDTH, EDGE_LABEL_FONT))
}

/// How far a node's capture label reaches above its top border, halo included,
/// or nothing when it captures no wire.
pub(super) fn capture_rise(names: &[String]) -> i32 {
    let Some(lines) = wrap_wires(names) else {
        return 0;
    };

    LABEL_RISE
        + (lines.len() as i32 - 1) * EDGE_LINE_HEIGHT
        + EDGE_LABEL_FONT / 2
        + 2 * EDGE_LABEL_HALO
}

/// Leaves enough room for the largest pair of hand-over and capture labels.
pub(super) fn vertical_gap(graph: &SemanticModel) -> i32 {
    // ponytail: one global gap keeps routing simple; reserve per-edge gaps if
    // tall diagrams become a practical problem.
    let source_lines = label_line_count(graph, &graph.flow.flow_inputs);
    let block_lines = graph.flow.blocks.iter().flat_map(|block| {
        // A question or choice hands over one output per connection, so its
        // gap follows the widest single name, not all of them joined.
        let handover = match block.kind {
            BlockKind::Question | BlockKind::Choice => block
                .outputs
                .iter()
                .map(|output| label_line_count(graph, [output]))
                .max()
                .unwrap_or(0),
            BlockKind::Action | BlockKind::End => label_line_count(graph, &block.outputs),
        };
        [
            handover,
            label_line_count(
                graph,
                block
                    .inputs
                    .iter()
                    .filter(|input| !input.borrowed)
                    .map(|input| &input.ident),
            ),
        ]
    });
    let lines = block_lines.chain([source_lines]).max().unwrap_or(0) as i32;
    if lines == 0 {
        return MIN_VERTICAL_GAP;
    }

    MIN_VERTICAL_GAP.max(
        LABEL_DROP
            + LABEL_RISE
            + EDGE_LABEL_FONT
            + 2 * EDGE_LABEL_HALO
            + 2 * (lines - 1) * EDGE_LINE_HEIGHT,
    )
}

fn label_line_count<'a>(
    graph: &SemanticModel,
    names: impl IntoIterator<Item = &'a Ident>,
) -> usize {
    wrap_wires(&drawn(graph, names)).map_or(0, |lines| lines.len())
}

/// Wraps one connection label and stacks its lines against `at`, or nothing
/// when that end names no wire. A label beside a horizontal run is also kept
/// right of `exit`: it reaches back over the node it left once it wraps, and
/// nodes are drawn last and would cover its first column.
fn wire_label(names: &[String], at: Point, stack: Stack, exit: Option<i32>) -> Option<Label> {
    let lines = wrap_wires(names)?;
    let below_first = (lines.len() as i32 - 1) * EDGE_LINE_HEIGHT;
    let clear = exit.map_or(at.x, |exit| {
        at.x.max(exit + label_width(&lines) / 2 + EDGE_LABEL_HALO)
    });

    Some(Label {
        at: Point {
            x: clear,
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

/// The middle of the run a connection leaves on, beside the wire rather than
/// over it, for the label both of its ends agree on.
fn centre_of(edge: &Edge) -> (Point, Option<i32>) {
    let [start, next] = route_head(edge);
    if start.y == next.y {
        return (
            Point {
                x: i32::midpoint(start.x, next.x),
                y: start.y - LABEL_RISE,
            },
            Some(start.x),
        );
    }

    (
        Point {
            x: start.x + LABEL_ASIDE,
            y: i32::midpoint(start.y, next.y) - LABEL_BASELINE,
        },
        None,
    )
}

/// Just outside the exit a connection leaves by, so its hand-over reads as
/// belonging to the node above it.
fn exit_of(edge: &Edge, origin_x: i32) -> (Point, Option<i32>) {
    let [start, next] = route_head(edge);
    // A run leaving a node's side, vertical or not, drops its label level with
    // that node, so both need the clamp: `origin_x` is the node's centre, and
    // an exit away from it is on a side rather than the bottom.
    if start.y == next.y || start.x != origin_x {
        return (
            Point {
                x: start.x,
                y: start.y + LABEL_DROP,
            },
            Some(start.x),
        );
    }

    (
        Point {
            x: start.x + LABEL_ASIDE,
            y: start.y + LABEL_DROP,
        },
        None,
    )
}

/// The first segment of a route, which is the one leaving the origin.
fn route_head(edge: &Edge) -> [Point; 2] {
    let [start, next, ..] = edge.points[..] else {
        unreachable!("a positioned connection has at least two points")
    };
    [start, next]
}

/// Right and bottom extent of a connection label, matching how the serializer
/// places it: centred on `at.x`, its first baseline at `at.y`.
pub(super) fn label_bounds(label: &Label) -> (i32, i32) {
    let width = label_width(&label.lines);
    let last_baseline = label.at.y + (label.lines.len() as i32 - 1) * EDGE_LINE_HEIGHT;

    (
        label.at.x + width / 2 + EDGE_LABEL_HALO,
        last_baseline + EDGE_LABEL_FONT / 2 + EDGE_LABEL_HALO,
    )
}

pub(super) fn label_width(lines: &[String]) -> i32 {
    lines
        .iter()
        .map(|line| text_width(&line.graphemes(true).collect::<Vec<_>>(), EDGE_LABEL_FONT))
        .max()
        .unwrap_or_default()
}
