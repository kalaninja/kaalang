use std::collections::HashMap;

use contour_model::{BlockKind, Branch, Convergence, Graph, Plan};
use unicode_segmentation::UnicodeSegmentation;

mod action;
mod choice;
mod convergence;
mod merge;
mod question;

const MARGIN: i32 = 32;
const SKEWER_WIDTH: i32 = 360;
const VERTICAL_GAP: i32 = 72;
const TERMINAL_STUB: i32 = 24;
/// Height of the collector every terminal branch enters, measured up from the
/// top edge of the return node.
const COLLECTOR_GAP: i32 = 32;
const RETURN_LANE_GAP: i32 = 52;
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
    Return,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum NodeKind {
    Start,
    Action,
    Question,
    Choice,
    Case,
    Merge,
    Return,
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

pub(crate) fn layout(graph: &Graph) -> Scene {
    let parameters = graph
        .flow
        .sources
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join(", ");
    let skewer_count = plan_span(&graph.plan);
    let mut builder = Builder {
        graph,
        scene: Scene {
            width: 0,
            height: 0,
            nodes: Vec::new(),
            edges: Vec::new(),
        },
        indexes: HashMap::new(),
        terminals: Vec::new(),
        skewer_count,
    };

    let start = builder.add_node(
        NodeId::Start,
        NodeKind::Start,
        format!("{}({parameters})", graph.name),
        0,
        MARGIN,
    );
    let first_top = builder.node_bottom(start) + VERTICAL_GAP;
    let placed = builder.place(
        &graph.plan,
        0,
        first_top,
        Incoming {
            origin: Origin::bottom(start),
            label: None,
            skewer: 0,
        },
    );
    let end = builder.add_node(
        NodeId::Return,
        NodeKind::Return,
        String::new(),
        0,
        placed.bottom + VERTICAL_GAP,
    );
    builder.connect_terminals(end);

    let case_count = graph
        .flow
        .blocks
        .iter()
        .map(|block| block.case_descriptions.len())
        .sum::<usize>();
    debug_assert_eq!(
        builder.scene.nodes.len(),
        graph.flow.blocks.len() + case_count + 2
    );

    builder.fit_scene();
    builder.scene
}

struct Builder<'a> {
    graph: &'a Graph,
    scene: Scene,
    indexes: HashMap<NodeId, usize>,
    terminals: Vec<Tail>,
    skewer_count: usize,
}

impl Builder<'_> {
    fn place(&mut self, plan: &Plan, skewer: usize, top: i32, incoming: Incoming) -> Placed {
        match plan {
            Plan::Action { index, next } => self.place_action(*index, next, skewer, top, incoming),
            Plan::Question {
                index,
                branches,
                merge,
                convergence,
            } => self.place_question(
                *index,
                branches,
                Join {
                    merge: merge.as_ref(),
                    convergence: convergence.as_ref(),
                },
                skewer,
                top,
                incoming,
            ),
            Plan::Choice {
                index,
                branches,
                merge,
                convergence,
            } => self.place_choice(
                *index,
                branches,
                Join {
                    merge: merge.as_ref(),
                    convergence: convergence.as_ref(),
                },
                skewer,
                top,
                incoming,
            ),
            Plan::Terminal { output } => {
                self.terminals.push(Tail {
                    origin: incoming.origin,
                    label: Some(output.to_string()),
                    skewer: incoming.skewer,
                    merge: None,
                });
                Placed {
                    bottom: self.anchor(incoming.origin).y,
                    arrivals: Vec::new(),
                }
            }
            Plan::Arrival { input, merge } => Placed {
                bottom: self.anchor(incoming.origin).y,
                arrivals: vec![Tail {
                    origin: incoming.origin,
                    label: Some(incoming.label.unwrap_or_else(|| input.to_string())),
                    skewer: incoming.skewer,
                    merge: Some(*merge),
                }],
            },
            Plan::Yield { wires } => Placed {
                bottom: self.anchor(incoming.origin).y,
                arrivals: vec![Tail {
                    origin: incoming.origin,
                    label: (!wires.is_empty()).then(|| {
                        wires
                            .iter()
                            .map(ToString::to_string)
                            .collect::<Vec<_>>()
                            .join(", ")
                    }),
                    skewer: incoming.skewer,
                    merge: None,
                }],
            },
        }
    }

    fn finish_branches(&mut self, placed: Vec<Placed>, join: Join<'_>, skewer: usize) -> Placed {
        let bottom = placed.iter().map(|branch| branch.bottom).max().unwrap_or(0);
        // Branch order decides which skewer a shared continuation takes, and a
        // nested branch point hands its own tails up in that same order.
        let arrivals = placed
            .into_iter()
            .flat_map(|branch| branch.arrivals)
            .collect::<Vec<_>>();
        if let Some(merge) = join.merge {
            return self.place_merge(arrivals, merge, skewer, bottom);
        }
        if let Some(convergence) = join.convergence {
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
                BlockKind::Merge => NodeKind::Merge,
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
    fn terminal_is_clear(&self, terminal: &Tail, end: NodeId, join_y: i32) -> bool {
        let start = self.anchor(terminal.origin);
        let end_top = self.top_anchor(end);
        let blocked_by_node = self.scene.nodes.iter().any(|node| {
            node.id != terminal.origin.node
                && node.id != end
                && node.x == start.x
                && self.node_top(node.id) > start.y
                && self.node_top(node.id) < end_top.y
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

    /// Draws every terminal into one horizontal collector above the return
    /// node, mirroring the distributor that fans a Select out to its cases.
    /// Each branch drops vertically onto the collector and the collector makes
    /// the single descent into the node, so the runs terminals share are one
    /// path rather than connections hidden behind each other.
    fn connect_terminals(&mut self, end: NodeId) {
        let end_top = self.top_anchor(end);
        let collector_y = end_top.y - COLLECTOR_GAP;
        let return_lane = self.return_lane_x();
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
                let lane = return_lane + detour * RETURN_LANE_GAP;
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
                let lane = (lane_x - return_lane) / RETURN_LANE_GAP;
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
            self.connect_points(
                terminal.origin.node,
                end,
                terminal.label,
                points,
                Some(Point {
                    x: start.x + 42,
                    y: start.y + 18,
                }),
            );
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

    fn node_top(&self, id: NodeId) -> i32 {
        self.top_anchor(id).y
    }

    fn node_bottom(&self, id: NodeId) -> i32 {
        self.bottom_anchor(id).y
    }

    fn node(&self, id: NodeId) -> &Node {
        &self.scene.nodes[self.indexes[&id]]
    }

    fn return_lane_x(&self) -> i32 {
        skewer_x(self.skewer_count - 1) + NODE_WIDTH / 2 + RETURN_LANE_GAP
    }

    fn fit_scene(&mut self) {
        let node_right = self.scene.nodes.iter().map(|node| node.x + node.width / 2);
        let edge_right = self
            .scene
            .edges
            .iter()
            .flat_map(|edge| edge.points.iter().map(|point| point.x));
        let label_right = self
            .scene
            .edges
            .iter()
            .filter_map(|edge| label_bounds(edge).map(|(right, _)| right));
        let node_bottom = self.scene.nodes.iter().map(|node| node.y + node.height / 2);
        let edge_bottom = self
            .scene
            .edges
            .iter()
            .flat_map(|edge| edge.points.iter().map(|point| point.y));
        let label_bottom = self
            .scene
            .edges
            .iter()
            .filter_map(|edge| label_bounds(edge).map(|(_, bottom)| bottom));

        self.scene.width = node_right
            .chain(edge_right)
            .chain(label_right)
            .max()
            .unwrap_or(0)
            + MARGIN;
        self.scene.height = node_bottom
            .chain(edge_bottom)
            .chain(label_bottom)
            .max()
            .unwrap_or(0)
            + MARGIN;
    }
}

struct Incoming {
    origin: Origin,
    label: Option<String>,
    skewer: usize,
}

#[derive(Clone, Copy)]
struct Join<'a> {
    merge: Option<&'a contour_model::Merge>,
    convergence: Option<&'a Convergence>,
}

struct Placed {
    bottom: i32,
    /// Tails still looking for the shared continuation they enter. A branch
    /// point that has no join of its own passes its branches' tails outward.
    arrivals: Vec<Tail>,
}

struct Tail {
    origin: Origin,
    label: Option<String>,
    skewer: usize,
    merge: Option<usize>,
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
        Plan::Action { next, .. } => plan_span(next),
        Plan::Question {
            branches,
            merge,
            convergence,
            ..
        } => branch_span(branches, merge.as_ref(), convergence.as_ref()),
        Plan::Choice {
            branches,
            merge,
            convergence,
            ..
        } => branch_span(branches, merge.as_ref(), convergence.as_ref()),
        Plan::Terminal { .. } | Plan::Arrival { .. } | Plan::Yield { .. } => 1,
    }
}

fn branch_span(
    branches: &[Branch],
    merge: Option<&contour_model::Merge>,
    convergence: Option<&Convergence>,
) -> usize {
    let spans = branches
        .iter()
        .map(|branch| plan_span(&branch.plan))
        .collect::<Vec<_>>();
    let total = spans.iter().sum::<usize>();
    let continuation = merge
        .map(|merge| merge.next.as_ref())
        .or_else(|| convergence.map(|convergence| convergence.next.as_ref()));
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
        NodeKind::Merge => merge::dimensions(),
        NodeKind::Return => (180, 58, Vec::new()),
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
        LABEL_FONT, MARGIN, NODE_LABEL_WIDTH, Node, NodeId, NodeKind, Point, Scene, label_bounds,
        layout, skewer_x, text_width, wrap_text,
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
        let graph = contour_model::build(function).expect("the flow is valid");

        layout(&graph)
    }

    fn node(scene: &Scene, id: NodeId) -> &Node {
        scene
            .nodes
            .iter()
            .find(|node| node.id == id)
            .expect("the node is drawn")
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

    #[test]
    fn canvas_follows_the_actual_scene_bounds() {
        let scene = scene(include_str!("../../tests/fixtures/all_blocks.rs"), "route");
        let content_right = scene
            .nodes
            .iter()
            .map(|node| node.x + node.width / 2)
            .chain(
                scene
                    .edges
                    .iter()
                    .flat_map(|edge| edge.points.iter().map(|point| point.x)),
            )
            .chain(
                scene
                    .edges
                    .iter()
                    .filter_map(|edge| label_bounds(edge).map(|(right, _)| right)),
            )
            .max()
            .expect("the scene is not empty");
        let content_bottom = scene
            .nodes
            .iter()
            .map(|node| node.y + node.height / 2)
            .chain(
                scene
                    .edges
                    .iter()
                    .flat_map(|edge| edge.points.iter().map(|point| point.y)),
            )
            .chain(
                scene
                    .edges
                    .iter()
                    .filter_map(|edge| label_bounds(edge).map(|(_, bottom)| bottom)),
            )
            .max()
            .expect("the scene is not empty");

        assert_eq!(scene.width, content_right + MARGIN);
        assert_eq!(scene.height, content_bottom + MARGIN);
    }

    /// Node geometry dominates the canvas at the current spacing constants, so
    /// this does not reproduce a past clipping bug. It pins the invariant that a
    /// wrapped connection label stays drawable, which the spacing constants
    /// currently satisfy only by a few pixels.
    #[test]
    fn a_wrapped_connection_label_stays_inside_the_canvas() {
        let source = r#"
            #[contour]
            fn wide(condition: bool) -> u8 {
                #[question("Choose a path")]
                |condition| -> (accepted, rejected) { condition };

                #[action("Take the accepted path")]
                |accepted| -> accepted_result { 1 };

                #[action("Take the rejected path")]
                |rejected| -> a_deliberately_long_wire_name_that_wraps_the_connection_label_onto_several_lines {
                    0
                };
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
    fn nested_branches_converge_at_their_own_merge() {
        use NodeId::{Block, Return, Start};

        let source = r#"
            #[contour]
            fn nested(outer: bool, inner: bool) -> u8 {
                #[question("Take the outer path?")]
                |outer| -> (outer_yes, outer_no) { outer };

                #[question("Take the inner path?")]
                |outer_yes, inner| -> (inner_yes, inner_no) { inner };

                #[action("Build the inner yes value")]
                |inner_yes| -> inner_yes_value { 1 };

                #[action("Build the inner no value")]
                |inner_no| -> inner_no_value { 2 };

                #[merge]
                |inner_yes_value, inner_no_value| -> inner_value {};

                #[action("Finish the inner path")]
                |inner_value| -> outer_yes_value { inner_value };

                #[action("Build the outer no value")]
                |outer_no| -> outer_no_value { 0 };

                #[merge]
                |outer_yes_value, outer_no_value| -> outer_value {};

                #[action("Return the result")]
                |outer_value| -> result { outer_value };
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
                (Block(4), Block(5)),
                (Block(0), Block(6)),
                (Block(5), Block(7)),
                (Block(6), Block(7)),
                (Block(7), Block(8)),
                (Block(8), Return),
            ]
        );
        assert_eq!(node(&scene, Block(0)).x, node(&scene, Block(1)).x);
        assert_eq!(node(&scene, Block(0)).x, node(&scene, Block(4)).x);
        assert_eq!(node(&scene, Block(0)).x, node(&scene, Block(7)).x);
        assert!(node(&scene, Block(3)).x < node(&scene, Block(6)).x);
    }

    #[test]
    fn implicit_convergence_places_the_shared_consumer_once() {
        use NodeId::{Block, Return, Start};

        let source = r#"
            #[contour]
            fn choose(condition: bool) -> u8 {
                #[question("Choose a path")]
                |condition| -> (yes, no) { condition };

                #[action("Build yes")]
                |yes| -> selected { 1 };

                #[action("Build no")]
                |no| -> selected { 2 };

                #[action("Use selected")]
                |selected| -> result { selected };
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
                (Block(3), Return),
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
    }

    #[test]
    fn nested_convergence_routes_every_branch_into_one_consumer() {
        use NodeId::{Block, Return, Start};

        let source = r#"
            #[contour]
            fn choose(outer: bool, inner: bool) -> u8 {
                #[question("Take the nested path?")]
                |outer| -> (nested, direct) { outer };

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
                (Block(5), Return),
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
        use NodeId::{Block, Return, Start};

        let source = r#"
            #[contour]
            fn choose(outer: bool, inner: bool) -> u8 {
                #[question("Take the nested path?")]
                |outer| -> (nested, direct) { outer };

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
                (Block(6), Return),
            ]
        );
    }

    #[test]
    fn choice_uses_ordered_case_nodes_and_output_only_labels() {
        use NodeId::{Block, Case};

        let source = r#"
            #[contour]
            fn choose(input: u8) -> u8 {
                #[choice("Choose a path")]
                #[case("Left")]
                #[case("Middle")]
                #[case("Right")]
                |input| -> (left, middle, right) {
                    match input { 0 => (), 1 => (), _ => () }
                };

                #[action("Build left")]
                |left| -> left_result { 0 };

                #[action("Build middle")]
                |middle| -> middle_result { 1 };

                #[action("Build right")]
                |right| -> right_result { 2 };
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
    fn a_terminal_sibling_reaches_the_end_beside_a_merge() {
        use NodeId::{Block, Case, Return, Start};

        let source = r#"
            #[contour]
            fn partial(input: u8) -> u8 {
                #[choice("Choose a path")]
                #[case("Left")]
                #[case("Right")]
                #[case("Finish now")]
                |input| -> (left, right, done) {
                    match input { 0 => (), 1 => (), _ => () }
                };

                #[action("Build left")]
                |left| -> left_value { 1 };

                #[action("Build right")]
                |right| -> right_value { 2 };

                #[action("Finish immediately")]
                |done| -> immediate_result { 3 };

                #[merge]
                |left_value, right_value| -> selected {};

                #[action("Finish after merge")]
                |selected| -> merged_result { selected };
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
                (Block(4), Block(5)),
                (Block(3), Return),
                (Block(5), Return),
            ]
        );
        let early_terminal = scene
            .edges
            .iter()
            .find(|edge| edge.from == Block(3) && edge.to == Return)
            .expect("the early terminal reaches Return");
        assert_eq!(early_terminal.points.len(), 4);
        assert_eq!(early_terminal.points[0].x, early_terminal.points[1].x);
        assert_eq!(early_terminal.points[1].x, node(&scene, Block(3)).x);
        assert_eq!(early_terminal.points[2].x, node(&scene, Return).x);
    }

    /// Terminal branches meet in one collector above the return node: each
    /// drops onto its shared row, and the collector makes the single descent
    /// into the node.
    #[test]
    fn terminal_branches_share_one_collector_into_the_return_node() {
        let source = r#"
            #[contour]
            fn partial(input: u8) -> u8 {
                #[choice("Choose a path")]
                #[case("Finish first")]
                #[case("Finish second")]
                #[case("Finish third")]
                #[case("Finish fourth")]
                |input| -> (first, second, third, fourth) {
                    match input { 0 => (), 1 => (), 2 => (), _ => () }
                };

                #[action("Build first")]
                |first| -> first_result { 1 };

                #[action("Build second")]
                |second| -> second_result { 2 };

                #[action("Build third")]
                |third| -> third_result { 3 };

                #[action("Build fourth")]
                |fourth| -> fourth_result { 4 };
            }
        "#;

        let scene = scene(source, "partial");
        let return_top = node(&scene, NodeId::Return);
        let return_top = Point {
            x: return_top.x,
            y: return_top.y - return_top.height / 2,
        };
        let terminals = scene
            .edges
            .iter()
            .filter(|edge| edge.to == NodeId::Return)
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
            .expect("the branch above the return node drops straight in");

        assert_eq!(turning.len(), 3);
        let collector_y = turning[0].points[turning[0].points.len() - 2].y;
        let descent = [
            Point {
                x: return_top.x,
                y: collector_y,
            },
            return_top,
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
        assert_eq!(straight.points[1], return_top);
        assert_eq!(straight.points[0].x, return_top.x);
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

    /// A branch that ends the flow may lead the ones that merge, and it keeps
    /// the branching block's own skewer. The merge cannot also sit there, so it
    /// and its continuation take the first continuing branch's skewer.
    #[test]
    fn a_leading_terminal_case_keeps_the_merge_off_its_skewer() {
        let source = r#"
            #[contour]
            fn partial(input: u8) -> u8 {
                #[choice("Choose a path")]
                #[case("Finish now")]
                #[case("Left")]
                #[case("Right")]
                |input| -> (done, left, right) {
                    match input { 0 => (), 1 => (), _ => () }
                };

                #[action("Finish immediately")]
                |done| -> immediate_result { 1 };

                #[action("Build left")]
                |left| -> left_value { 2 };

                #[action("Build right")]
                |right| -> right_value { 3 };

                #[merge]
                |left_value, right_value| -> selected {};

                #[action("Finish after merge")]
                |selected| -> merged_result { selected };
            }
        "#;

        let scene = scene(source, "partial");
        let merge = scene
            .nodes
            .iter()
            .find(|node| node.kind == NodeKind::Merge)
            .expect("the merge is drawn");

        assert_eq!(merge.x, skewer_x(1));
        assert_eq!(node(&scene, NodeId::Block(1)).x, skewer_x(0));
        assert_eq!(node(&scene, NodeId::Block(5)).x, skewer_x(1));
    }

    /// Branch order alone cannot free every terminal column. A continuation
    /// wider than the branches beside it reaches past them, and the terminal it
    /// covers is routed outside the continuing branches instead.
    #[test]
    fn a_terminal_under_a_wide_continuation_uses_the_outer_lane() {
        use NodeId::{Block, Return};

        let source = r#"
            #[contour]
            fn partial(input: u8) -> u8 {
                #[choice("Choose a path")]
                #[case("Left")]
                #[case("Right")]
                #[case("Finish now")]
                |input| -> (left, right, done) {
                    match input { 0 => (), 1 => (), _ => () }
                };

                #[action("Build left")]
                |left| -> left_value { 1 };

                #[action("Build right")]
                |right| -> right_value { 2 };

                #[action("Finish immediately")]
                |done| -> immediate_result { 3 };

                #[merge]
                |left_value, right_value| -> selected {};

                #[choice("Widen the continuation")]
                #[case("Wide left")]
                #[case("Wide middle")]
                #[case("Wide right")]
                |selected| -> (wide_left, wide_middle, wide_right) {
                    match selected { 0 => (), 1 => (), _ => () }
                };

                #[action("Build wide left")]
                |wide_left| -> wide_left_result { 4 };

                #[action("Build wide middle")]
                |wide_middle| -> wide_middle_result { 5 };

                #[action("Build wide right")]
                |wide_right| -> wide_right_result { 6 };
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
            .find(|edge| edge.from == Block(3) && edge.to == Return)
            .expect("the early terminal reaches Return");

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

                #[action("Return accepted")]
                |accepted| -> accepted_result { 1 };

                #[action("Return rejected")]
                |rejected| -> rejected_result { 0 };
            }
        };
        let graph = contour_model::build(&function).expect("the flow is valid");
        let scene = layout(&graph);
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
    /// into the return node share the collector row and its descent.
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
        let ([first, second], ..) = (segment,) else {
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
