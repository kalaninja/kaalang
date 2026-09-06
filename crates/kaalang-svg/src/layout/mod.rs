use std::collections::HashMap;

use kaalang_model::{BlockKind, Branch, ExecutionPlan, Join, JoinTarget, SemanticModel};
use syn::{Ident, Signature, spanned::Spanned};

mod action;
mod choice;
mod convergence;
mod dependency;
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

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
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

pub(crate) fn layout(graph: &SemanticModel, signature: &str) -> Scene {
    let mut builder = Builder {
        graph,
        scene: Scene::default(),
        indexes: HashMap::new(),
        arrivals: Vec::new(),
        vertical_gap: vertical_gap(graph),
    };
    if dependency::is_graph(&graph.execution_plan) {
        return dependency::layout(builder, signature);
    }

    let start = builder.add_node(
        NodeId::Start,
        NodeKind::Start,
        signature.to_owned(),
        0,
        MARGIN,
    );
    let first_top = builder.bottom_anchor(start).y + builder.vertical_gap;
    builder.place(
        &graph.execution_plan,
        0,
        first_top,
        Incoming {
            origin: Origin::bottom(start),
            branch: None,
            skewer: 0,
        },
    );

    debug_assert!(
        builder.arrivals.is_empty(),
        "end is placed outside every path, so its arrivals are all drawn by now"
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
    graph: &'a SemanticModel,
    scene: Scene,
    indexes: HashMap<NodeId, usize>,
    /// The `result` hand-overs waiting for the end node, in branch order.
    arrivals: Vec<Incoming>,
    vertical_gap: i32,
}

impl Builder<'_> {
    fn place(
        &mut self,
        plan: &ExecutionPlan,
        skewer: usize,
        top: i32,
        incoming: Incoming,
    ) -> Placed {
        match plan {
            ExecutionPlan::Guarded { .. } => unreachable!("guarded plans use dependency layout"),
            ExecutionPlan::End { index, body, .. } => {
                self.place_end(*index, body, skewer, top, incoming)
            }
            ExecutionPlan::Action { index, next } => {
                self.place_action(*index, next, skewer, top, incoming)
            }
            ExecutionPlan::Question {
                index,
                branches,
                join,
            } => self.place_question(*index, branches, join.as_ref(), skewer, top, incoming),
            ExecutionPlan::Choice {
                index,
                branches,
                joins,
            } => self.place_choice(*index, branches, single_join(joins), skewer, top, incoming),
            ExecutionPlan::EndArrival { .. } => {
                // Every branch hands the `result` wire over here. Alternative
                // producers meet at the junction end.rs draws above the node.
                self.arrivals.push(incoming);
                Placed {
                    bottom: self.anchor(incoming.origin).y,
                    arrivals: Vec::new(),
                }
            }
            ExecutionPlan::Yield { join, .. } => Placed {
                bottom: self.anchor(incoming.origin).y,
                arrivals: vec![Arrival {
                    incoming,
                    join: *join,
                }],
            },
        }
    }

    fn finish_branches(
        &mut self,
        index: usize,
        placed: Vec<Placed>,
        convergence: Option<&Join>,
    ) -> Placed {
        let bottom = placed.iter().map(|branch| branch.bottom).max().unwrap_or(0);
        // Branch order decides which skewer a shared continuation takes, and a
        // nested branch point hands its own arrivals up in that same order.
        let arrivals = placed
            .into_iter()
            .flat_map(|branch| branch.arrivals)
            .collect::<Vec<_>>();
        if let Some(convergence) = convergence {
            let target = JoinTarget {
                block: index,
                join: 0,
            };
            let (local, mut outward): (Vec<_>, Vec<_>) = arrivals
                .into_iter()
                .partition(|arrival| arrival.join == target);
            let local = local.into_iter().map(|arrival| arrival.incoming).collect();
            let mut placed = self.place_convergence(local, convergence, bottom);
            outward.append(&mut placed.arrivals);
            outward.sort_by_key(|arrival| arrival.incoming.skewer);
            placed.arrivals = outward;
            return placed;
        }
        // Every branch ended the flow, or they all converge further out.
        Placed { bottom, arrivals }
    }

    fn add_block_node(&mut self, index: usize, skewer: usize, top: i32) -> NodeId {
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
            NodeId::Start => self.graph.flow.flow_inputs.iter().collect(),
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
    arrivals: Vec<Arrival>,
}

/// One yielded connection waiting for its specific enclosing join.
struct Arrival {
    incoming: Incoming,
    join: JoinTarget,
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

struct PlanMetrics {
    span: usize,
    /// Outward yields and their skewer offsets, in skewer order.
    yields: Vec<(JoinTarget, usize)>,
}

fn plan_metrics(plan: &ExecutionPlan) -> PlanMetrics {
    match plan {
        ExecutionPlan::Guarded { .. } => unreachable!("guarded plans use dependency layout"),
        ExecutionPlan::End { body, .. } => plan_metrics(body),
        ExecutionPlan::Action { next, .. } => plan_metrics(next),
        ExecutionPlan::Question {
            index,
            branches,
            join,
        } => branch_metrics(*index, branches, join.as_ref()).1,
        ExecutionPlan::Choice {
            index,
            branches,
            joins,
        } => branch_metrics(*index, branches, single_join(joins)).1,
        ExecutionPlan::EndArrival { .. } => PlanMetrics {
            span: 1,
            yields: Vec::new(),
        },
        ExecutionPlan::Yield { join, .. } => PlanMetrics {
            span: 1,
            yields: vec![(*join, 0)],
        },
    }
}

/// The skewer layout draws one join per choice; plan 5 replaces it with a
/// layout that draws every convergence group.
fn single_join(joins: &[Join]) -> Option<&Join> {
    match joins {
        [] => None,
        [join] => Some(join),
        _ => unimplemented!("several convergence groups of one choice are drawn by plan 5"),
    }
}

fn branch_layout(
    index: usize,
    branches: &[Branch],
    convergence: Option<&Join>,
) -> (Vec<usize>, usize) {
    let (offsets, metrics) = branch_metrics(index, branches, convergence);
    (offsets, metrics.span)
}

fn branch_metrics(
    index: usize,
    branches: &[Branch],
    convergence: Option<&Join>,
) -> (Vec<usize>, PlanMetrics) {
    let children = branches
        .iter()
        .map(|branch| plan_metrics(&branch.plan))
        .collect::<Vec<_>>();
    let mut offsets = Vec::with_capacity(children.len());
    let mut total = 0;
    for child in &children {
        offsets.push(total);
        total += child.span;
    }
    let Some(convergence) = convergence else {
        let yields = children
            .iter()
            .zip(&offsets)
            .flat_map(|(child, offset)| {
                child
                    .yields
                    .iter()
                    .map(move |(join, skewer)| (*join, offset + skewer))
            })
            .collect();
        return (
            offsets,
            PlanMetrics {
                span: total,
                yields,
            },
        );
    };

    let target = JoinTarget {
        block: index,
        join: 0,
    };
    let continues = |child: &PlanMetrics| child.yields.iter().any(|(join, _)| *join == target);
    let first = children
        .iter()
        .position(continues)
        .expect("a convergence has a continuing branch");
    let last = children
        .iter()
        .rposition(continues)
        .expect("a convergence has a continuing branch");

    let continuing_span = children[first..=last]
        .iter()
        .map(|child| child.span)
        .sum::<usize>();
    // The continuation is drawn on the skewer the first arrival reaches, which
    // lies inside the first continuing branch rather than on its own skewer
    // when that branch yields from within a nested block.
    let arrival = children[first]
        .yields
        .iter()
        .find_map(|(join, skewer)| (*join == target).then_some(*skewer))
        .expect("a continuing branch yields");
    let continuation = plan_metrics(&convergence.next);
    let reserved = (arrival + continuation.span).saturating_sub(continuing_span);
    for offset in &mut offsets[last + 1..] {
        *offset += reserved;
    }

    let mut yields = children
        .iter()
        .zip(&offsets)
        .flat_map(|(child, offset)| {
            child
                .yields
                .iter()
                .map(move |(join, skewer)| (*join, offset + skewer))
        })
        .filter(|(join, _)| *join != target)
        .chain(
            continuation
                .yields
                .into_iter()
                .map(|(join, skewer)| (join, offsets[first] + arrival + skewer)),
        )
        .collect::<Vec<_>>();
    yields.sort_by_key(|(_, skewer)| *skewer);
    (
        offsets,
        PlanMetrics {
            span: total + reserved,
            yields,
        },
    )
}

/// Renders every ordinary wire and every underscore-prefixed wire that a block
/// uses. Only an unused ignored wire is absent from the flow.
fn drawn<'a>(graph: &SemanticModel, names: impl IntoIterator<Item = &'a Ident>) -> Vec<String> {
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
