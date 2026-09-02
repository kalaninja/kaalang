use std::collections::HashMap;

use kaalang_model::{BlockKind, Branch, Convergence, Graph, Plan};
use syn::{Ident, Signature, spanned::Spanned};
use unicode_segmentation::UnicodeSegmentation;

mod action;
mod choice;
mod convergence;
mod end;
mod question;

const MARGIN: i32 = 32;
const SKEWER_WIDTH: i32 = 360;
const VERTICAL_GAP: i32 = 72;
const TERMINAL_STUB: i32 = 24;
/// Height of the collector every terminal branch enters, measured up from the
/// top edge of End.
const COLLECTOR_GAP: i32 = 32;
const END_LANE_GAP: i32 = 52;
const NODE_WIDTH: i32 = 280;
const CASE_WIDTH: i32 = 240;
pub(crate) const CASE_TIP_HEIGHT: i32 = 18;
pub(crate) const QUESTION_POINT: i32 = 28;
pub(crate) const SELECT_SKEW: i32 = 24;
/// Font size of a node label. The serializer writes the stylesheet from this,
/// so measurement and rendering cannot disagree.
pub(crate) const LABEL_FONT: i32 = 14;
/// Font size of an edge label, written into the stylesheet the same way.
pub(crate) const EDGE_LABEL_FONT: i32 = 12;
/// Baseline-to-baseline distance between the lines of a node label.
pub(crate) const LINE_HEIGHT: i32 = 18;
/// Baseline-to-baseline distance between the lines of an edge label.
pub(crate) const EDGE_LINE_HEIGHT: i32 = 14;
/// Width of the halo an edge label paints behind itself to stay readable where
/// it crosses a connection. Also written into the stylesheet.
pub(crate) const EDGE_LABEL_HALO: i32 = 5;
/// Text budget inside a rectangular node.
const NODE_LABEL_WIDTH: i32 = NODE_WIDTH - 32;
/// Branch icons lose horizontal space to their slanted sides.
const BRANCH_LABEL_WIDTH: i32 = NODE_WIDTH - 80;
/// Text budget inside a case icon.
const CASE_LABEL_WIDTH: i32 = CASE_WIDTH - 32;
/// Text budget for an edge label, which floats free of any node.
pub(crate) const EDGE_LABEL_WIDTH: i32 = 240;

#[derive(Default)]
pub(crate) struct Scene {
    pub(crate) width: i32,
    pub(crate) height: i32,
    pub(crate) nodes: Vec<Node>,
    pub(crate) edges: Vec<Edge>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum NodeId {
    Start,
    Block(usize),
    Case { choice: usize, branch: usize },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum NodeKind {
    Start,
    Action,
    Question,
    Choice,
    Case,
    End,
}

pub(crate) struct Node {
    pub(crate) id: NodeId,
    pub(crate) kind: NodeKind,
    pub(crate) label: String,
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

pub(crate) struct Edge {
    pub(crate) from: NodeId,
    pub(crate) to: NodeId,
    /// The exact authored wire name this connection carries.
    pub(crate) label: Option<String>,
    /// `label` wrapped to its budget, so its size is known during layout.
    pub(crate) lines: Vec<String>,
    pub(crate) points: Vec<Point>,
    pub(crate) label_at: Option<Point>,
}

/// The authored flow signature, minus `fn`, with every whitespace run collapsed
/// so a signature written across source lines wraps on the label's own terms.
pub(crate) fn signature_text(source: &str, signature: &Signature) -> String {
    let authored = source[signature.span().byte_range()]
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    authored.strip_prefix("fn ").unwrap_or(&authored).to_owned()
}

pub(crate) fn layout(graph: &Graph, signature: &str) -> Scene {
    let skewer_count = plan_span(&graph.plan);
    let mut builder = Builder {
        graph,
        scene: Scene::default(),
        indexes: HashMap::new(),
        terminals: Vec::new(),
        skewer_count,
    };

    let start = builder.add_node(
        NodeId::Start,
        NodeKind::Start,
        signature.to_owned(),
        0,
        MARGIN,
    );
    let first_top = builder.bottom_anchor(start).y + VERTICAL_GAP;
    builder.place(
        &graph.plan,
        0,
        first_top,
        Incoming {
            origin: Origin::bottom(start),
            label: None,
            skewer: 0,
        },
    );

    let case_count = graph
        .flow
        .blocks
        .iter()
        .map(|block| block.case_descriptions.len())
        .sum::<usize>();
    debug_assert_eq!(
        builder.scene.nodes.len(),
        graph.flow.blocks.len() + case_count + 1
    );

    builder.fit_scene();
    builder.scene
}

struct Builder<'a> {
    graph: &'a Graph,
    scene: Scene,
    indexes: HashMap<NodeId, usize>,
    terminals: Vec<Incoming>,
    skewer_count: usize,
}

impl Builder<'_> {
    fn place(&mut self, plan: &Plan, skewer: usize, top: i32, incoming: Incoming) -> Placed {
        match plan {
            Plan::End { index, body } => self.place_end(*index, body, skewer, top, incoming),
            Plan::Action { index, next } => self.place_action(*index, next, skewer, top, incoming),
            Plan::Question {
                index,
                branches,
                convergence,
            } => self.place_question(
                *index,
                branches,
                convergence.as_ref(),
                skewer,
                top,
                incoming,
            ),
            Plan::Choice {
                index,
                branches,
                convergence,
            } => self.place_choice(
                *index,
                branches,
                convergence.as_ref(),
                skewer,
                top,
                incoming,
            ),
            Plan::EndArrival { inputs } => {
                // Start reaches End only across a wire End captures: a boundary
                // that hands nothing over is drawn unconnected, whatever it
                // declares.
                let unwired_boundary = inputs.is_empty() && incoming.origin.node == NodeId::Start;
                if !unwired_boundary {
                    self.terminals.push(Incoming {
                        label: (!inputs.is_empty()).then(|| join(inputs)),
                        ..incoming
                    });
                }
                Placed {
                    bottom: self.anchor(incoming.origin).y,
                    arrivals: Vec::new(),
                }
            }
            Plan::Yield { wires } => Placed {
                bottom: self.anchor(incoming.origin).y,
                arrivals: vec![Incoming {
                    label: (!wires.is_empty()).then(|| join(wires)),
                    ..incoming
                }],
            },
        }
    }

    fn finish_branches(
        &mut self,
        placed: Vec<Placed>,
        convergence: Option<&Convergence>,
    ) -> Placed {
        let bottom = placed.iter().map(|branch| branch.bottom).max().unwrap_or(0);
        // Branch order decides which skewer a shared continuation takes, and a
        // nested branch point hands its own arrivals up in that same order.
        let arrivals = placed
            .into_iter()
            .flat_map(|branch| branch.arrivals)
            .collect::<Vec<_>>();
        if let Some(convergence) = convergence {
            return self.place_convergence(arrivals, convergence, bottom);
        }
        // Every branch ended the flow, or they all converge further out.
        Placed { bottom, arrivals }
    }

    fn add_authored_node(&mut self, index: usize, skewer: usize, top: i32) -> NodeId {
        let block = &self.graph.flow.blocks[index];
        self.add_node(
            NodeId::Block(index),
            match block.kind {
                BlockKind::Action => NodeKind::Action,
                BlockKind::Question => NodeKind::Question,
                BlockKind::Choice => NodeKind::Choice,
                BlockKind::End => NodeKind::End,
            },
            block.description.clone().unwrap_or_default(),
            skewer,
            top,
        )
    }

    fn add_node(
        &mut self,
        id: NodeId,
        kind: NodeKind,
        label: String,
        skewer: usize,
        top: i32,
    ) -> NodeId {
        debug_assert!(!self.indexes.contains_key(&id));
        let (width, height, lines) = node_dimensions(kind, &label);
        let index = self.scene.nodes.len();
        self.scene.nodes.push(Node {
            id,
            kind,
            label,
            x: skewer_x(skewer),
            y: top + height / 2,
            width,
            height,
            lines,
        });
        self.indexes.insert(id, index);
        id
    }

    fn connect_to_node(&mut self, incoming: Incoming, to: NodeId) {
        let start = self.anchor(incoming.origin);
        let end = self.top_anchor(to);
        let (points, label_at) = match incoming.origin.side {
            Side::Right => {
                let bend = Point {
                    x: end.x,
                    y: start.y,
                };
                (
                    compact_points([start, bend, end]),
                    incoming.label.as_ref().map(|_| Point {
                        x: i32::midpoint(start.x, bend.x),
                        y: start.y - 8,
                    }),
                )
            }
            Side::Bottom if start.x == end.x => (
                vec![start, end],
                incoming.label.as_ref().map(|_| Point {
                    x: start.x + 42,
                    y: i32::midpoint(start.y, end.y) - 5,
                }),
            ),
            Side::Bottom => {
                let middle_y = i32::midpoint(start.y, end.y);
                (
                    compact_points([
                        start,
                        Point {
                            x: start.x,
                            y: middle_y,
                        },
                        Point {
                            x: end.x,
                            y: middle_y,
                        },
                        end,
                    ]),
                    incoming.label.as_ref().map(|_| Point {
                        x: i32::midpoint(start.x, end.x),
                        y: middle_y - 8,
                    }),
                )
            }
        };
        self.connect_points(incoming.origin.node, to, incoming.label, points, label_at);
    }

    /// Reports whether a terminal can drop straight down its own column, which
    /// it cannot when a node or an earlier connection stands in the way.
    fn terminal_is_clear(&self, terminal: &Incoming, end: NodeId, join_y: i32) -> bool {
        let start = self.anchor(terminal.origin);
        let end_top = self.top_anchor(end);
        let blocked_by_node = self.scene.nodes.iter().any(|node| {
            let top = self.top_anchor(node.id).y;
            node.id != terminal.origin.node
                && node.id != end
                && node.x == start.x
                && top > start.y
                && top < end_top.y
        });
        // Every lane is chosen before the first terminal connection is drawn,
        // so the scene holds no terminal edges yet and none obstructs another.
        let blocked_by_edge = self.scene.edges.iter().any(|edge| {
            edge.points
                .windows(2)
                .any(|segment| vertical_route_hits(segment, start.x, start.y, join_y))
        });

        !blocked_by_node && !blocked_by_edge
    }

    /// Draws every terminal into one horizontal collector above End, mirroring
    /// the distributor that fans a Select out to its cases.
    /// Each branch drops vertically onto the collector and the collector makes
    /// the single descent into the node, so the runs terminals share are one
    /// path rather than connections hidden behind each other.
    fn connect_terminals(&mut self, end: NodeId) {
        let end_top = self.top_anchor(end);
        let collector_y = end_top.y - COLLECTOR_GAP;
        let end_lane = self.end_lane_x();
        let terminals = std::mem::take(&mut self.terminals);

        // The column each terminal drops down: its own when nothing blocks it,
        // an outer lane when something does. A detour needs its own lane, or
        // two of them trace the same one and hide each other on the way down.
        let mut detour = 0;
        let lanes = terminals
            .iter()
            .map(|terminal| {
                if self.terminal_is_clear(terminal, end, collector_y) {
                    return self.anchor(terminal.origin).x;
                }
                let lane = end_lane + detour * END_LANE_GAP;
                detour += 1;
                lane
            })
            .collect::<Vec<_>>();

        for (terminal, lane_x) in terminals.into_iter().zip(lanes) {
            let start = self.anchor(terminal.origin);
            let points = if lane_x == start.x && start.x == end_top.x {
                // The collector is degenerate for a branch already above the
                // node, exactly as the distributor is for a case below a Select.
                vec![start, end_top]
            } else if lane_x == start.x {
                compact_points([
                    start,
                    Point {
                        x: start.x,
                        y: collector_y,
                    },
                    Point {
                        x: end_top.x,
                        y: collector_y,
                    },
                    end_top,
                ])
            } else {
                // Detours leave on stepped rows, so an inner one crosses under
                // the next instead of running along it out to the lanes.
                let lane = (lane_x - end_lane) / END_LANE_GAP;
                let stub_y = start.y + TERMINAL_STUB * (detour - lane);
                compact_points([
                    start,
                    Point {
                        x: start.x,
                        y: stub_y,
                    },
                    Point {
                        x: lane_x,
                        y: stub_y,
                    },
                    Point {
                        x: lane_x,
                        y: collector_y,
                    },
                    Point {
                        x: end_top.x,
                        y: collector_y,
                    },
                    end_top,
                ])
            };
            let label_at = terminal.label.as_ref().map(|_| Point {
                x: start.x + 42,
                y: start.y + 18,
            });
            self.connect_points(terminal.origin.node, end, terminal.label, points, label_at);
        }
    }

    fn connect_points(
        &mut self,
        from: NodeId,
        to: NodeId,
        label: Option<String>,
        points: Vec<Point>,
        label_at: Option<Point>,
    ) {
        debug_assert!(points.len() >= 2);
        debug_assert_eq!(label.is_some(), label_at.is_some());
        let lines = label
            .as_deref()
            .map(|label| wrap_text(label, EDGE_LABEL_WIDTH, EDGE_LABEL_FONT))
            .unwrap_or_default();
        self.scene.edges.push(Edge {
            from,
            to,
            label,
            lines,
            points,
            label_at,
        });
    }

    fn anchor(&self, origin: Origin) -> Point {
        match origin.side {
            Side::Bottom => self.bottom_anchor(origin.node),
            Side::Right => self.right_anchor(origin.node),
        }
    }

    fn top_anchor(&self, id: NodeId) -> Point {
        let node = self.node(id);
        Point {
            x: node.x,
            y: node.y - node.height / 2,
        }
    }

    fn bottom_anchor(&self, id: NodeId) -> Point {
        let node = self.node(id);
        Point {
            x: node.x,
            y: node.y + node.height / 2,
        }
    }

    fn right_anchor(&self, id: NodeId) -> Point {
        let node = self.node(id);
        Point {
            x: node.x + node.width / 2,
            y: node.y,
        }
    }

    fn node(&self, id: NodeId) -> &Node {
        &self.scene.nodes[self.indexes[&id]]
    }

    fn end_lane_x(&self) -> i32 {
        skewer_x(self.skewer_count - 1) + NODE_WIDTH / 2 + END_LANE_GAP
    }

    fn fit_scene(&mut self) {
        let (mut right, mut bottom) = (0, 0);
        for node in &self.scene.nodes {
            right = right.max(node.x + node.width / 2);
            bottom = bottom.max(node.y + node.height / 2);
        }
        for edge in &self.scene.edges {
            for point in &edge.points {
                right = right.max(point.x);
                bottom = bottom.max(point.y);
            }
            if let Some((label_right, label_bottom)) = label_bounds(edge) {
                right = right.max(label_right);
                bottom = bottom.max(label_bottom);
            }
        }
        self.scene.width = right + MARGIN;
        self.scene.height = bottom + MARGIN;
    }
}

/// A connection that has left its origin and awaits the node it enters.
struct Incoming {
    origin: Origin,
    label: Option<String>,
    skewer: usize,
}

struct Placed {
    bottom: i32,
    /// Connections still looking for the shared continuation they enter. A
    /// branch point that has no join of its own hands them outward.
    arrivals: Vec<Incoming>,
}

#[derive(Clone, Copy)]
struct Origin {
    node: NodeId,
    side: Side,
}

impl Origin {
    const fn bottom(node: NodeId) -> Self {
        Self {
            node,
            side: Side::Bottom,
        }
    }

    const fn right(node: NodeId) -> Self {
        Self {
            node,
            side: Side::Right,
        }
    }
}

#[derive(Clone, Copy)]
enum Side {
    Bottom,
    Right,
}

fn plan_span(plan: &Plan) -> usize {
    match plan {
        Plan::End { body, .. } => plan_span(body),
        Plan::Action { next, .. } => plan_span(next),
        Plan::Question {
            branches,
            convergence,
            ..
        } => branch_span(branches, convergence.as_ref()),
        Plan::Choice {
            branches,
            convergence,
            ..
        } => branch_span(branches, convergence.as_ref()),
        Plan::EndArrival { .. } | Plan::Yield { .. } => 1,
    }
}

fn branch_span(branches: &[Branch], convergence: Option<&Convergence>) -> usize {
    let spans = branches
        .iter()
        .map(|branch| plan_span(&branch.plan))
        .collect::<Vec<_>>();
    let total = spans.iter().sum::<usize>();
    let continuation = convergence.map(|convergence| convergence.next.as_ref());
    let Some(continuation) = continuation else {
        return total;
    };

    // The shared continuation sits on the first continuing branch's skewer, so
    // it reaches beyond any branch that leads to it.
    total.max(continuation_offset(branches, &spans) + plan_span(continuation))
}

/// Skewers between a branching block and the first of its branches that reaches
/// the shared continuation, which is where that continuation is drawn.
fn continuation_offset(branches: &[Branch], spans: &[usize]) -> usize {
    let continuing = branches
        .iter()
        .position(|branch| !branch.early_return)
        .unwrap_or(0);

    spans[..continuing].iter().sum()
}

/// Right and bottom extent of an edge label, matching how the serializer places
/// it: centred on `label_at`, with the block of lines centred on that baseline.
fn label_bounds(edge: &Edge) -> Option<(i32, i32)> {
    let at = edge.label_at?;
    let width = edge
        .lines
        .iter()
        .map(|line| text_width(&line.graphemes(true).collect::<Vec<_>>(), EDGE_LABEL_FONT))
        .max()?;
    let last_baseline = at.y + (edge.lines.len() as i32 - 1) * EDGE_LINE_HEIGHT / 2;

    Some((
        at.x + width / 2 + EDGE_LABEL_HALO,
        last_baseline + EDGE_LABEL_FONT / 2 + EDGE_LABEL_HALO,
    ))
}

fn skewer_x(skewer: usize) -> i32 {
    MARGIN + NODE_WIDTH / 2 + skewer as i32 * SKEWER_WIDTH
}

/// Renders wire names as one comma-separated label.
fn join(names: &[Ident]) -> String {
    names
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join(", ")
}

fn compact_points(points: impl IntoIterator<Item = Point>) -> Vec<Point> {
    let mut points = Vec::from_iter(points);
    points.dedup();
    points
}

fn vertical_route_hits(segment: &[Point], x: i32, top: i32, bottom: i32) -> bool {
    let [from, to] = segment else {
        return false;
    };
    if from.x == to.x {
        from.x == x && from.y.max(to.y) > top && from.y.min(to.y) <= bottom
    } else {
        debug_assert_eq!(from.y, to.y);
        from.y > top && from.y <= bottom && from.x.min(to.x) <= x && x <= from.x.max(to.x)
    }
}

fn node_dimensions(kind: NodeKind, label: &str) -> (i32, i32, Vec<String>) {
    match kind {
        NodeKind::End => end::dimensions(),
        NodeKind::Start => {
            let lines = wrap_text(label, NODE_LABEL_WIDTH, LABEL_FONT);
            let height = 58.max(30 + lines.len() as i32 * LINE_HEIGHT);
            (NODE_WIDTH, height, lines)
        }
        NodeKind::Action => action::dimensions(label),
        NodeKind::Question | NodeKind::Choice => {
            block_dimensions(label, NODE_WIDTH, BRANCH_LABEL_WIDTH, 72)
        }
        NodeKind::Case => choice::case_dimensions(label),
    }
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

/// Approximate advance width of one character, in hundredths of an em, for the
/// sans-serif stack the stylesheet requests. The renderer has no font metrics,
/// so this estimate errs wide rather than letting a label leave its node.
fn advance(character: char) -> i32 {
    match character {
        '\t' => 200,
        ' ' | '.' | ',' | ':' | ';' | '!' | '|' | '\'' | '`' | 'i' | 'j' | 'l' | 'I' | '('
        | ')' | '[' | ']' | '{' | '}' | '/' | '\\' | '-' => 32,
        'm' | 'w' | 'M' | 'W' | '@' => 90,
        'A'..='Z' => 68,
        _ if character.is_ascii() => 56,
        // Latin-1, Greek, and Cyrillic behave like Latin; assume anything
        // beyond them, such as CJK, is full width.
        _ if (character as u32) < 0x0500 => 60,
        _ => 100,
    }
}

/// Estimated rendered width of one grapheme cluster. A cluster draws as a
/// single glyph however many code points spell it, so its base character
/// carries the estimate and the marks joined to it add nothing.
fn cluster_advance(cluster: &str) -> i32 {
    advance(
        cluster
            .chars()
            .next()
            .expect("a grapheme cluster has at least one character"),
    )
}

/// Estimated rendered width of `clusters` at `font_size`.
fn text_width(clusters: &[&str], font_size: i32) -> i32 {
    clusters.iter().copied().map(cluster_advance).sum::<i32>() * font_size / 100
}

/// Returns how many leading clusters fit within `budget`, at least one so that
/// wrapping always makes progress.
fn fitting_count(clusters: &[&str], budget: i32, font_size: i32) -> usize {
    let mut used = 0;
    for (count, cluster) in clusters.iter().enumerate() {
        used += cluster_advance(cluster);
        if used * font_size / 100 > budget {
            return count.max(1);
        }
    }

    clusters.len()
}

/// Breaks a label into lines that fit `budget` pixels at `font_size`, without
/// changing the authored text. A line breaks at the last space that fits, or
/// between grapheme clusters when no space does, so a word too long for the
/// budget is split rather than left to run outside its node. An authored line
/// break starts a new line and belongs to none of them, so concatenating the
/// result reproduces only a label that had no line breaks of its own.
pub(crate) fn wrap_text(text: &str, budget: i32, font_size: i32) -> Vec<String> {
    let mut lines = Vec::new();
    for paragraph in text.split('\n') {
        let clusters = paragraph.graphemes(true).collect::<Vec<_>>();
        if clusters.is_empty() {
            lines.push(String::new());
            continue;
        }

        let mut start = 0;
        while text_width(&clusters[start..], font_size) > budget {
            let hard_end = start + fitting_count(&clusters[start..], budget, font_size);
            let preferred_break = clusters[start..hard_end]
                .iter()
                .rposition(|cluster| cluster.starts_with(char::is_whitespace))
                .map(|offset| start + offset + 1)
                .filter(|end| *end > start + 1);
            // A space past the budget is no break at all: taking it would draw
            // the line outside the node the budget measures.
            let end = preferred_break.unwrap_or(hard_end);
            lines.push(clusters[start..end].concat());
            start = end;
        }
        if start < clusters.len() {
            lines.push(clusters[start..].concat());
        }
    }

    lines
}

#[cfg(test)]
mod tests {
    use syn::{ItemFn, parse_quote};
    use unicode_segmentation::UnicodeSegmentation;

    use super::{
        LABEL_FONT, NODE_LABEL_WIDTH, Node, NodeId, NodeKind, Point, Scene, label_bounds, layout,
        signature_text, skewer_x, text_width, wrap_text,
    };

    fn scene(source: &str, flow: &str) -> Scene {
        let file = syn::parse_file(source).expect("the fixture parses");
        let syn::Item::Fn(function) = file
            .items
            .iter()
            .find(|item| matches!(item, syn::Item::Fn(function) if function.sig.ident == flow))
            .expect("the fixture declares the flow")
        else {
            unreachable!("the item was matched as a function")
        };
        let graph = kaalang_model::build(function).expect("the flow is valid");

        layout(&graph, &signature_text(source, &function.sig))
    }

    fn node(scene: &Scene, id: NodeId) -> &Node {
        scene
            .nodes
            .iter()
            .find(|node| node.id == id)
            .expect("the node is drawn")
    }

    /// Start carries the signature as authored. `fn` goes because the label is
    /// already known to be the flow, a signature spread over source lines
    /// becomes one line so the label wraps to its own budget, and everything
    /// else survives: a wildcard parameter the boundary declares no wire for,
    /// a raw identifier, generics, and a where clause.
    #[test]
    fn a_signature_label_keeps_the_authored_text_without_the_fn_keyword() {
        let source = r"
            #[kaalang]
            fn boundary<T>(
                _: u8,
                r#type: T,
            ) -> T
            where
                T: Clone,
            {
                #[end]
                |r#type| {};
            }
        ";
        let file = syn::parse_file(source).expect("the fixture parses");
        let syn::Item::Fn(function) = &file.items[0] else {
            unreachable!("the fixture declares a function")
        };

        assert_eq!(
            signature_text(source, &function.sig),
            "boundary<T>( _: u8, r#type: T, ) -> T where T: Clone,"
        );
    }

    /// A word with no break opportunity is split rather than left to run
    /// outside its node, but never inside a grapheme cluster: a cluster draws
    /// as one glyph, and splitting it would render different text.
    #[test]
    fn wrapping_splits_between_graphemes_and_never_inside_one() {
        let word = "драконоподобный";
        let lines = wrap_text(word, 20, 14);

        assert!(lines.len() > 1);
        assert_eq!(lines.concat(), word);
        for line in &lines {
            let clusters = line.graphemes(true).collect::<Vec<_>>();
            assert!(text_width(&clusters, 14) <= 20, "line exceeds: {line}");
        }

        // Narrower than one cluster, so only the cluster boundary can break.
        assert_eq!(wrap_text("👩‍💻👩‍💻", 8, 14), ["👩‍💻", "👩‍💻"]);
        assert_eq!(
            wrap_text("e\u{301}e\u{301}", 8, 14),
            ["e\u{301}", "e\u{301}"]
        );
    }

    /// Start reaches End only across a wire End captures. What the boundary
    /// declares does not decide it: a wildcard parameter, a named parameter
    /// left unconsumed, and no parameter at all are drawn the same way.
    #[test]
    fn a_zero_computation_flow_connects_start_to_end_only_through_a_captured_wire() {
        for parameters in ["", "_: u8", "_value: u8"] {
            let source = format!(
                r"
                #[kaalang]
                fn boundary({parameters}) {{
                    #[end]
                    || {{}};
                }}
            "
            );
            let scene = scene(&source, "boundary");

            assert_eq!(
                scene.nodes.iter().map(|node| node.id).collect::<Vec<_>>(),
                [NodeId::Start, NodeId::Block(0)]
            );
            assert_eq!(node(&scene, NodeId::Block(0)).kind, NodeKind::End);
            assert!(
                scene.edges.is_empty(),
                "`{parameters}` hands nothing to End, so nothing connects them"
            );
        }

        let scene = scene(
            r"
                #[kaalang]
                fn identity<T>(value: T) -> T {
                    #[end]
                    |value| {};
                }
            ",
            "identity",
        );

        assert_eq!(node(&scene, NodeId::Block(0)).kind, NodeKind::End);
        assert_eq!(
            scene
                .edges
                .iter()
                .map(|edge| (edge.from, edge.to, edge.label.as_deref()))
                .collect::<Vec<_>>(),
            [(NodeId::Start, NodeId::Block(0), Some("value"))]
        );
    }

    /// Node geometry dominates the canvas at the current spacing constants, so
    /// this does not reproduce a past clipping bug. It pins the invariant that a
    /// wrapped connection label stays drawable, which the spacing constants
    /// currently satisfy only by a few pixels.
    #[test]
    fn a_wrapped_connection_label_stays_inside_the_canvas() {
        let source = r#"
            #[kaalang]
            fn wide(condition: bool) -> u8 {
                #[question("Choose a path")]
                |condition| -> (accepted, rejected) { condition };

                #[action("Take the accepted path")]
                |accepted| -> a_deliberately_long_wire_name_that_wraps_the_connection_label_onto_several_lines { 1 };

                #[action("Take the rejected path")]
                |rejected| -> a_deliberately_long_wire_name_that_wraps_the_connection_label_onto_several_lines {
                    0
                };

                #[end]
                |a_deliberately_long_wire_name_that_wraps_the_connection_label_onto_several_lines| {};
            }
        "#;

        let scene = scene(source, "wide");
        let labelled = scene
            .edges
            .iter()
            .filter_map(label_bounds)
            .collect::<Vec<_>>();

        assert!(!labelled.is_empty());
        assert!(
            scene.edges.iter().any(|edge| edge.lines.len() > 1),
            "the fixture must wrap at least one connection label"
        );
        for (right, bottom) in labelled {
            assert!(right <= scene.width, "a label leaves the canvas: {right}");
            assert!(
                bottom <= scene.height,
                "a label leaves the canvas: {bottom}"
            );
        }
    }

    #[test]
    fn wrapping_preserves_authored_whitespace() {
        let text = "  exact  spacing  ";

        assert_eq!(wrap_text(text, 40, 14).concat(), text);
    }

    #[test]
    fn wrapped_lines_fit_the_label_budget() {
        let label = "REJECT THE WWWWIDE APPLICATION IMMEDIATELY AND WITHOUT DELAY";
        let lines = wrap_text(label, NODE_LABEL_WIDTH, LABEL_FONT);

        assert!(lines.len() > 1);
        for line in &lines {
            let clusters = line.graphemes(true).collect::<Vec<_>>();
            assert!(
                text_width(&clusters, LABEL_FONT) <= NODE_LABEL_WIDTH,
                "line exceeds the budget: {line}"
            );
        }
        assert_eq!(lines.concat(), label);
    }

    /// The space after a long word sits past the budget, so breaking there
    /// would draw the word and the space outside the node.
    #[test]
    fn wrapping_splits_a_long_word_rather_than_reaching_the_space_after_it() {
        let text = format!("{} tail", "W".repeat(32));
        let lines = wrap_text(&text, NODE_LABEL_WIDTH, LABEL_FONT);

        assert!(lines.len() > 1);
        assert_eq!(lines.concat(), text);
        for line in &lines {
            let clusters = line.graphemes(true).collect::<Vec<_>>();
            assert!(
                text_width(&clusters, LABEL_FONT) <= NODE_LABEL_WIDTH,
                "line exceeds the budget: {line}"
            );
        }
    }

    #[test]
    fn nested_branches_converge_at_their_own_consumers() {
        use NodeId::{Block, Start};

        let source = r#"
            #[kaalang]
            fn nested(outer: bool, inner: bool) -> u8 {
                #[question("Take the outer path?")]
                |outer, &inner| -> (outer_yes, outer_no) { outer };

                #[question("Take the inner path?")]
                |outer_yes, inner| -> (inner_yes, inner_no) { inner };

                #[action("Build the inner yes value")]
                |inner_yes| -> inner_value { 1 };

                #[action("Build the inner no value")]
                |inner_no| -> inner_value { 2 };

                #[action("Produce the inner-path value")]
                |inner_value| -> outer_value { inner_value };

                #[action("Build the outer no value")]
                |outer_no| -> outer_value { 0 };

                #[action("Produce the result")]
                |outer_value| -> result { outer_value };

                #[end]
                |result| {};
            }
        "#;

        let scene = scene(source, "nested");
        assert_eq!(
            scene
                .edges
                .iter()
                .map(|edge| (edge.from, edge.to))
                .collect::<Vec<_>>(),
            [
                (Start, Block(0)),
                (Block(0), Block(1)),
                (Block(1), Block(2)),
                (Block(1), Block(3)),
                (Block(2), Block(4)),
                (Block(3), Block(4)),
                (Block(0), Block(5)),
                (Block(4), Block(6)),
                (Block(5), Block(6)),
                (Block(6), Block(7)),
            ]
        );
        assert_eq!(node(&scene, Block(0)).x, node(&scene, Block(1)).x);
        assert_eq!(node(&scene, Block(0)).x, node(&scene, Block(4)).x);
        assert_eq!(node(&scene, Block(0)).x, node(&scene, Block(6)).x);
        assert!(node(&scene, Block(3)).x < node(&scene, Block(5)).x);
    }

    #[test]
    fn implicit_convergence_places_the_shared_consumer_once() {
        use NodeId::{Block, Start};

        let source = r#"
            #[kaalang]
            fn choose(condition: bool) -> u8 {
                #[question("Choose a path")]
                |condition| -> (yes, no) { condition };

                #[action("Build yes")]
                |yes| -> selected { 1 };

                #[action("Build no")]
                |no| -> selected { 2 };

                #[action("Use selected")]
                |selected| -> result { selected };

                #[end]
                |result| {};
            }
        "#;

        let scene = scene(source, "choose");
        assert_eq!(
            scene
                .edges
                .iter()
                .map(|edge| (edge.from, edge.to))
                .collect::<Vec<_>>(),
            [
                (Start, Block(0)),
                (Block(0), Block(1)),
                (Block(0), Block(2)),
                (Block(1), Block(3)),
                (Block(2), Block(3)),
                (Block(3), Block(4)),
            ]
        );
        assert_eq!(
            scene
                .nodes
                .iter()
                .filter(|node| node.id == Block(3))
                .count(),
            1
        );
        assert_eq!(
            scene
                .edges
                .iter()
                .filter(|edge| edge.to == Block(3))
                .map(|edge| edge.label.as_deref())
                .collect::<Vec<_>>(),
            [Some("selected"), Some("selected")]
        );
    }

    #[test]
    fn nested_convergence_routes_every_branch_into_one_consumer() {
        use NodeId::{Block, Start};

        let source = r#"
            #[kaalang]
            fn choose(outer: bool, inner: bool) -> u8 {
                #[question("Take the nested path?")]
                |outer, &inner| -> (nested, direct) { outer };

                #[question("Choose the nested value")]
                |nested, inner| -> (inner_yes, inner_no) { inner };

                #[action("Build nested yes")]
                |inner_yes| -> selected { 1 };

                #[action("Build nested no")]
                |inner_no| -> selected { 2 };

                #[action("Build direct")]
                |direct| -> selected { 3 };

                #[action("Use selected")]
                |selected| -> result { selected };

                #[end]
                |result| {};
            }
        "#;

        let scene = scene(source, "choose");
        assert_eq!(
            scene
                .edges
                .iter()
                .map(|edge| (edge.from, edge.to))
                .collect::<Vec<_>>(),
            [
                (Start, Block(0)),
                (Block(0), Block(1)),
                (Block(1), Block(2)),
                (Block(1), Block(3)),
                (Block(0), Block(4)),
                (Block(2), Block(5)),
                (Block(3), Block(5)),
                (Block(4), Block(5)),
                (Block(5), Block(6)),
            ]
        );
        assert_eq!(
            scene
                .nodes
                .iter()
                .filter(|node| node.id == Block(5))
                .count(),
            1
        );
        // The shared consumer takes the first continuing branch's skewer.
        assert_eq!(node(&scene, Block(2)).x, node(&scene, Block(5)).x);
        for edge in &scene.edges {
            for segment in edge.points.windows(2) {
                for node in &scene.nodes {
                    assert!(
                        !enters_node(segment, node),
                        "{:?} crosses {:?}",
                        edge.from,
                        node.id
                    );
                }
            }
        }
    }

    #[test]
    fn a_nested_branch_point_without_its_own_join_hands_its_tails_outward() {
        use NodeId::{Block, Start};

        let source = r#"
            #[kaalang]
            fn choose(outer: bool, inner: bool) -> u8 {
                #[question("Take the nested path?")]
                |outer, &inner| -> (nested, direct) { outer };

                #[question("Choose the nested depth")]
                |nested, inner| -> (short, long) { inner };

                #[action("Build the short value")]
                |short| -> selected { 1 };

                #[action("Prepare the long value")]
                |long| -> prepared { 2 };

                #[action("Build the direct value")]
                |direct| -> selected { 3 };

                #[action("Build the long value")]
                |prepared| -> selected { 4 };

                #[action("Use selected")]
                |selected| -> result { selected };

                #[end]
                |result| {};
            }
        "#;

        let scene = scene(source, "choose");
        assert_eq!(
            scene
                .edges
                .iter()
                .map(|edge| (edge.from, edge.to))
                .collect::<Vec<_>>(),
            [
                (Start, Block(0)),
                (Block(0), Block(1)),
                (Block(1), Block(2)),
                (Block(1), Block(3)),
                (Block(3), Block(5)),
                (Block(0), Block(4)),
                (Block(2), Block(6)),
                (Block(5), Block(6)),
                (Block(4), Block(6)),
                (Block(6), Block(7)),
            ]
        );
    }

    #[test]
    fn choice_uses_ordered_case_nodes_and_output_only_labels() {
        use NodeId::{Block, Case};

        let source = r#"
            #[kaalang]
            fn choose(input: u8) -> u8 {
                #[choice("Choose a path")]
                #[case("Left")]
                #[case("Middle")]
                #[case("Right")]
                |input| -> (left, middle, right) {
                    match input { 0 => (), 1 => (), _ => () }
                };

                #[action("Build left")]
                |left| -> result { 0 };

                #[action("Build middle")]
                |middle| -> result { 1 };

                #[action("Build right")]
                |right| -> result { 2 };

                #[end]
                |result| {};
            }
        "#;
        let scene = scene(source, "choose");
        let select_edges = scene
            .edges
            .iter()
            .filter(|edge| edge.from == Block(0))
            .collect::<Vec<_>>();

        assert_eq!(select_edges.len(), 3);
        assert!(select_edges.iter().all(|edge| edge.label.is_none()));
        assert_eq!(
            select_edges.iter().map(|edge| edge.to).collect::<Vec<_>>(),
            [
                Case {
                    choice: 0,
                    branch: 0
                },
                Case {
                    choice: 0,
                    branch: 1
                },
                Case {
                    choice: 0,
                    branch: 2
                },
            ]
        );
        let case_edges = scene
            .edges
            .iter()
            .filter(|edge| matches!(edge.from, Case { .. }))
            .collect::<Vec<_>>();
        assert_eq!(
            case_edges
                .iter()
                .map(|edge| edge.label.as_deref())
                .collect::<Vec<_>>(),
            [Some("left"), Some("middle"), Some("right")]
        );
        let case_x = scene
            .nodes
            .iter()
            .filter(|node| node.kind == NodeKind::Case)
            .map(|node| node.x)
            .collect::<Vec<_>>();
        assert!(case_x.windows(2).all(|pair| pair[0] < pair[1]));
    }

    #[test]
    fn a_terminal_sibling_reaches_the_end_beside_a_convergence() {
        use NodeId::{Block, Case, Start};

        let source = r#"
            #[kaalang]
            fn partial(input: u8) -> u8 {
                #[choice("Choose a path")]
                #[case("Left")]
                #[case("Right")]
                #[case("Reach End directly")]
                |input| -> (left, right, done) {
                    match input { 0 => (), 1 => (), _ => () }
                };

                #[action("Build left")]
                |left| -> selected { 1 };

                #[action("Build right")]
                |right| -> selected { 2 };

                #[action("Produce the direct result")]
                |done| -> result { 3 };

                #[action("Produce the converged result")]
                |selected| -> result { selected };

                #[end]
                |result| {};
            }
        "#;

        let scene = scene(source, "partial");
        assert_eq!(
            scene
                .edges
                .iter()
                .map(|edge| (edge.from, edge.to))
                .collect::<Vec<_>>(),
            [
                (Start, Block(0)),
                (
                    Block(0),
                    Case {
                        choice: 0,
                        branch: 0
                    }
                ),
                (
                    Block(0),
                    Case {
                        choice: 0,
                        branch: 1
                    }
                ),
                (
                    Block(0),
                    Case {
                        choice: 0,
                        branch: 2
                    }
                ),
                (
                    Case {
                        choice: 0,
                        branch: 0
                    },
                    Block(1)
                ),
                (
                    Case {
                        choice: 0,
                        branch: 1
                    },
                    Block(2)
                ),
                (
                    Case {
                        choice: 0,
                        branch: 2
                    },
                    Block(3)
                ),
                (Block(1), Block(4)),
                (Block(2), Block(4)),
                (Block(3), Block(5)),
                (Block(4), Block(5)),
            ]
        );
        let early_terminal = scene
            .edges
            .iter()
            .find(|edge| edge.from == Block(3) && edge.to == Block(5))
            .expect("the early terminal reaches End");
        assert_eq!(early_terminal.points.len(), 4);
        assert_eq!(early_terminal.points[0].x, early_terminal.points[1].x);
        assert_eq!(early_terminal.points[1].x, node(&scene, Block(3)).x);
        assert_eq!(early_terminal.points[2].x, node(&scene, Block(5)).x);
    }

    /// Terminal branches meet in one collector above End: each
    /// drops onto its shared row, and the collector makes the single descent
    /// into the node.
    #[test]
    fn terminal_branches_share_one_collector_into_end() {
        let source = r#"
            #[kaalang]
            fn partial(input: u8) -> u8 {
                #[choice("Choose a path")]
                #[case("First End path")]
                #[case("Second End path")]
                #[case("Third End path")]
                #[case("Fourth End path")]
                |input| -> (first, second, third, fourth) {
                    match input { 0 => (), 1 => (), 2 => (), _ => () }
                };

                #[action("Build first")]
                |first| -> result { 1 };

                #[action("Build second")]
                |second| -> result { 2 };

                #[action("Build third")]
                |third| -> result { 3 };

                #[action("Build fourth")]
                |fourth| -> result { 4 };

                #[end]
                |result| {};
            }
        "#;

        let scene = scene(source, "partial");
        let end_top = node(&scene, NodeId::Block(5));
        let end_top = Point {
            x: end_top.x,
            y: end_top.y - end_top.height / 2,
        };
        let terminals = scene
            .edges
            .iter()
            .filter(|edge| edge.to == NodeId::Block(5))
            .collect::<Vec<_>>();

        assert_eq!(terminals.len(), 4);

        // Every branch away from the column turns onto the collector, and the
        // collector makes one descent that the branch already in the column
        // drops straight through.
        let turning = terminals
            .iter()
            .filter(|edge| edge.points.len() > 2)
            .collect::<Vec<_>>();
        let straight = terminals
            .iter()
            .find(|edge| edge.points.len() == 2)
            .expect("the branch above End drops straight in");

        assert_eq!(turning.len(), 3);
        let collector_y = turning[0].points[turning[0].points.len() - 2].y;
        let descent = [
            Point {
                x: end_top.x,
                y: collector_y,
            },
            end_top,
        ];
        for edge in &turning {
            let corner = edge.points[edge.points.len() - 2];
            assert_eq!(
                corner.y, collector_y,
                "{:?} leaves the collector",
                edge.from
            );
            assert_eq!(
                [corner, edge.points[edge.points.len() - 1]],
                descent,
                "{:?} makes its own descent",
                edge.from
            );
        }
        assert_eq!(straight.points[1], end_top);
        assert_eq!(straight.points[0].x, end_top.x);
        assert!(straight.points[0].y <= collector_y);

        for edge in terminals {
            for segment in edge.points.windows(2) {
                for node in &scene.nodes {
                    assert!(
                        !enters_node(segment, node),
                        "{:?} crosses {:?}: {:?}",
                        edge.from,
                        node.id,
                        edge.points
                    );
                }
            }
        }
    }

    /// A branch that ends the flow may lead the ones that converge, and it keeps
    /// the branching block's own skewer. The shared continuation takes the
    /// first continuing branch's skewer.
    #[test]
    fn a_leading_end_path_keeps_the_shared_continuation_off_its_skewer() {
        let source = r#"
            #[kaalang]
            fn partial(input: u8) -> u8 {
                #[choice("Choose a path")]
                #[case("Reach End directly")]
                #[case("Left")]
                #[case("Right")]
                |input| -> (done, left, right) {
                    match input { 0 => (), 1 => (), _ => () }
                };

                #[action("Produce the direct result")]
                |done| -> result { 1 };

                #[action("Build left")]
                |left| -> selected { 2 };

                #[action("Build right")]
                |right| -> selected { 3 };

                #[action("Produce the converged result")]
                |selected| -> result { selected };

                #[end]
                |result| {};
            }
        "#;

        let scene = scene(source, "partial");
        assert_eq!(node(&scene, NodeId::Block(1)).x, skewer_x(0));
        assert_eq!(node(&scene, NodeId::Block(4)).x, skewer_x(1));
    }

    /// Branch order alone cannot free every terminal column. A continuation
    /// wider than the branches beside it reaches past them, and the terminal it
    /// covers is routed outside the continuing branches instead.
    #[test]
    fn a_terminal_under_a_wide_continuation_uses_the_outer_lane() {
        use NodeId::Block;

        let source = r#"
            #[kaalang]
            fn partial(input: u8) -> u8 {
                #[choice("Choose a path")]
                #[case("Left")]
                #[case("Right")]
                #[case("Reach End directly")]
                |input| -> (left, right, done) {
                    match input { 0 => (), 1 => (), _ => () }
                };

                #[action("Build left")]
                |left| -> selected { 1 };

                #[action("Build right")]
                |right| -> selected { 2 };

                #[action("Produce the direct result")]
                |done| -> result { 3 };

                #[choice("Widen the continuation")]
                #[case("Wide left")]
                #[case("Wide middle")]
                #[case("Wide right")]
                |selected| -> (wide_left, wide_middle, wide_right) {
                    match selected { 0 => (), 1 => (), _ => () }
                };

                #[action("Build wide left")]
                |wide_left| -> result { 4 };

                #[action("Build wide middle")]
                |wide_middle| -> result { 5 };

                #[action("Build wide right")]
                |wide_right| -> result { 6 };

                #[end]
                |result| {};
            }
        "#;

        let scene = scene(source, "partial");
        let rightmost_node = scene
            .nodes
            .iter()
            .map(|node| node.x + node.width / 2)
            .max()
            .expect("the scene is not empty");
        let early_terminal = scene
            .edges
            .iter()
            .find(|edge| edge.from == Block(3) && edge.to == Block(8))
            .expect("the early terminal reaches End");

        assert!(
            early_terminal
                .points
                .iter()
                .any(|point| point.x > rightmost_node)
        );
    }

    #[test]
    fn question_keeps_the_first_output_vertical_and_the_second_to_the_right() {
        let function: ItemFn = parse_quote! {
            fn decide(condition: bool) -> u8 {
                #[question("Choose a path")]
                |condition| -> (accepted, rejected) { condition };

                #[action("Produce the accepted result")]
                |accepted| -> result { 1 };

                #[action("Produce the rejected result")]
                |rejected| -> result { 0 };

                #[end]
                |result| {};
            }
        };
        let graph = kaalang_model::build(&function).expect("the flow is valid");
        let scene = layout(&graph, "decide(condition: bool) -> u8");
        let question = scene
            .nodes
            .iter()
            .find(|node| node.kind == NodeKind::Question)
            .expect("the question is drawn");
        let edges = scene
            .edges
            .iter()
            .filter(|edge| edge.from == NodeId::Block(0))
            .collect::<Vec<_>>();

        assert_eq!(edges.len(), 2);
        assert_eq!(edges[0].label.as_deref(), Some("accepted"));
        assert_eq!(edges[0].points[0].x, question.x);
        assert_eq!(edges[0].points[0].y, question.y + question.height / 2);
        assert_eq!(edges[1].label.as_deref(), Some("rejected"));
        assert_eq!(edges[1].points[0].x, question.x + question.width / 2);
        assert_eq!(edges[1].points[0].y, question.y);
        assert!(edges[1].points[1].x > question.x);
    }

    /// `segments_cross` compares a vertical run against a horizontal one, so
    /// collinear overlap is out of scope. Two overlaps are deliberate: the
    /// connections from a Select share the distributor row, and the connections
    /// into End share the collector row and its descent.
    #[test]
    fn fixture_connections_are_orthogonal_and_free_of_perpendicular_crossings() {
        let scene = scene(include_str!("../../tests/fixtures/all_blocks.rs"), "route");
        for edge in &scene.edges {
            assert!(
                edge.points
                    .windows(2)
                    .all(|points| points[0].x == points[1].x || points[0].y == points[1].y)
            );
        }

        for (index, left) in scene.edges.iter().enumerate() {
            for right in &scene.edges[index + 1..] {
                if left.from == right.from || left.to == right.to {
                    continue;
                }
                assert!(!left.points.windows(2).any(|left| {
                    right
                        .points
                        .windows(2)
                        .any(|right| segments_cross(left[0], left[1], right[0], right[1]))
                }));
            }
        }
    }

    /// Reports whether an axis-aligned segment passes through a node's interior.
    /// Anchors sit on the boundary, so only a strict crossing counts.
    fn enters_node(segment: &[Point], node: &Node) -> bool {
        let (left, right) = (node.x - node.width / 2, node.x + node.width / 2);
        let (top, bottom) = (node.y - node.height / 2, node.y + node.height / 2);
        let [first, second] = segment else {
            return false;
        };
        if first.x == second.x {
            left < first.x
                && first.x < right
                && first.y.min(second.y) < bottom
                && top < first.y.max(second.y)
        } else {
            top < first.y
                && first.y < bottom
                && first.x.min(second.x) < right
                && left < first.x.max(second.x)
        }
    }

    fn segments_cross(first: Point, second: Point, third: Point, fourth: Point) -> bool {
        let (vertical_start, vertical_end, horizontal_start, horizontal_end) =
            if first.x == second.x && third.y == fourth.y {
                (first, second, third, fourth)
            } else if first.y == second.y && third.x == fourth.x {
                (third, fourth, first, second)
            } else {
                return false;
            };
        let horizontal_min = horizontal_start.x.min(horizontal_end.x);
        let horizontal_max = horizontal_start.x.max(horizontal_end.x);
        let vertical_min = vertical_start.y.min(vertical_end.y);
        let vertical_max = vertical_start.y.max(vertical_end.y);

        horizontal_min < vertical_start.x
            && vertical_start.x < horizontal_max
            && vertical_min < horizontal_start.y
            && horizontal_start.y < vertical_max
    }
}
