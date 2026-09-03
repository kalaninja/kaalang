use std::collections::HashMap;

use kaalang_model::{BlockKind, Branch, Convergence, Graph, Plan};
use syn::{Ident, Signature, spanned::Spanned};

mod action;
mod choice;
mod convergence;
mod end;
mod label;
mod question;
#[cfg(test)]
mod tests;
mod text;

use label::{label_bounds, vertical_gap};
use text::wrap_text;

const MARGIN: i32 = 32;
const SKEWER_WIDTH: i32 = 360;
const MIN_VERTICAL_GAP: i32 = 72;
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

#[derive(Default)]
pub(crate) struct Scene {
    pub(crate) width: i32,
    pub(crate) height: i32,
    pub(crate) nodes: Vec<Node>,
    pub(crate) edges: Vec<Edge>,
    pub(crate) labels: Vec<Label>,
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
    /// The logical wire names the origin hands over here.
    pub(crate) handover: Vec<String>,
    /// The logical wire names the destination captures.
    pub(crate) capture: Vec<String>,
    pub(crate) points: Vec<Point>,
}

/// One placed connection label, wrapped during layout so the canvas can be
/// sized around it. `at` is the first line's baseline, so the serializer writes
/// the block without deciding where it sits.
pub(crate) struct Label {
    pub(crate) lines: Vec<String>,
    pub(crate) at: Point,
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
        vertical_gap: vertical_gap(graph),
    };

    let start = builder.add_node(
        NodeId::Start,
        NodeKind::Start,
        signature.to_owned(),
        0,
        MARGIN,
    );
    let first_top = builder.bottom_anchor(start).y + builder.vertical_gap;
    builder.place(
        &graph.plan,
        0,
        first_top,
        Incoming {
            origin: Origin::bottom(start),
            branch: None,
            skewer: 0,
        },
    );

    debug_assert!(
        builder.terminals.is_empty(),
        "End is placed outside every path, so its arrivals are all drawn by now"
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

    builder.place_labels();
    builder.fit_scene();
    builder.scene
}

struct Builder<'a> {
    graph: &'a Graph,
    scene: Scene,
    indexes: HashMap<NodeId, usize>,
    terminals: Vec<Incoming>,
    skewer_count: usize,
    vertical_gap: i32,
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
                    self.terminals.push(incoming);
                }
                Placed {
                    bottom: self.anchor(incoming.origin).y,
                    arrivals: Vec::new(),
                }
            }
            Plan::Yield { .. } => Placed {
                bottom: self.anchor(incoming.origin).y,
                arrivals: vec![incoming],
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
        let points = match incoming.origin.side {
            Side::Right => compact_points([
                start,
                Point {
                    x: end.x,
                    y: start.y,
                },
                end,
            ]),
            Side::Bottom if start.x == end.x => vec![start, end],
            Side::Bottom => {
                let middle_y = i32::midpoint(start.y, end.y);
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
                ])
            }
        };
        self.connect(incoming, to, points);
    }

    /// Records one connection with the wire names each of its ends names, which
    /// `place_labels` turns into the drawn labels once every node is placed.
    fn connect(&mut self, incoming: Incoming, to: NodeId, points: Vec<Point>) {
        let handover = self.handover(incoming.origin.node, incoming.branch);
        let capture = self.capture(to);
        self.connect_points(incoming.origin.node, to, handover, capture, points);
    }

    fn connect_points(
        &mut self,
        from: NodeId,
        to: NodeId,
        handover: Vec<String>,
        capture: Vec<String>,
        points: Vec<Point>,
    ) {
        debug_assert!(points.len() >= 2);
        self.scene.edges.push(Edge {
            from,
            to,
            handover,
            capture,
            points,
        });
    }

    /// The wire names a node hands over on one connection leaving it. A
    /// question hands over the single output its branch carries; a case hands
    /// over the choice output it stands for; every other node hands over all it
    /// produces.
    fn handover(&self, from: NodeId, branch: Option<usize>) -> Vec<String> {
        let names: Vec<&Ident> = match from {
            NodeId::Start => self.graph.flow.sources.iter().collect(),
            NodeId::Case {
                choice,
                branch: case,
            } => {
                vec![&self.graph.flow.blocks[choice].outputs[case]]
            }
            NodeId::Block(index) => {
                let outputs = &self.graph.flow.blocks[index].outputs;
                match branch {
                    Some(branch) => vec![&outputs[branch]],
                    None => outputs.iter().collect(),
                }
            }
        };

        drawn(self.graph, names)
    }

    /// The wire names a node captures. Only consuming inputs count: a borrow
    /// reads its wire where it lies and leaves it on the flow, so it is a data
    /// dependency, which the visual graph does not draw. A case captures
    /// nothing of its own: it stands for one output of the choice above it.
    fn capture(&self, to: NodeId) -> Vec<String> {
        match to {
            NodeId::Start | NodeId::Case { .. } => Vec::new(),
            NodeId::Block(index) => drawn(
                self.graph,
                self.graph.flow.blocks[index]
                    .inputs
                    .iter()
                    .filter(|input| !input.borrowed)
                    .map(|input| &input.ident),
            ),
        }
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
        }
        for label in &self.scene.labels {
            let (label_right, label_bottom) = label_bounds(label);
            right = right.max(label_right);
            bottom = bottom.max(label_bottom);
        }
        self.scene.width = right + MARGIN;
        self.scene.height = bottom + MARGIN;
    }
}

/// A connection that has left its origin and awaits the node it enters.
#[derive(Clone, Copy)]
struct Incoming {
    origin: Origin,
    /// The branch this connection left on, which decides how much of a
    /// branching origin's output tuple it hands over. `None` for a node that
    /// hands over everything it produces.
    branch: Option<usize>,
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

/// Renders every ordinary wire and every underscore-prefixed wire that a block
/// uses. Only an unused ignored wire is absent from the flow.
fn drawn<'a>(graph: &Graph, names: impl IntoIterator<Item = &'a Ident>) -> Vec<String> {
    names
        .into_iter()
        .filter(|name| {
            !name.to_string().starts_with('_')
                || graph
                    .flow
                    .blocks
                    .iter()
                    .any(|block| block.inputs.iter().any(|input| input.ident == **name))
        })
        .map(ToString::to_string)
        .collect()
}

fn skewer_x(skewer: usize) -> i32 {
    MARGIN + NODE_WIDTH / 2 + skewer as i32 * SKEWER_WIDTH
}

fn compact_points(points: impl IntoIterator<Item = Point>) -> Vec<Point> {
    let mut points = Vec::from_iter(points);
    points.dedup();
    points
}

fn node_dimensions(kind: NodeKind, label: &str) -> (i32, i32, Vec<String>) {
    match kind {
        NodeKind::End => end::dimensions(),
        NodeKind::Start => block_dimensions(label, NODE_WIDTH, NODE_LABEL_WIDTH, 58),
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
