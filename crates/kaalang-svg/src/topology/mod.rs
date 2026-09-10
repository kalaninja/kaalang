//! Projects the validated semantic model into the diagram's visual topology:
//! its nodes, exits, implicit junctions and connections, before any
//! coordinate exists.
//!
//! RFC 0002 §7 defines the drawn connections as the union, over every possible
//! execution, of the direct precedence between participating nodes. This module
//! preserves the model's dependencies and merges, reading its verified
//! execution plan for the same source order codegen emits.

use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet};

use kaalang_model::{
    Block, BlockKind, Execution, ExecutionOutcome, ExecutionPlan, Input, ProducerId, SemanticModel,
};
use syn::{FnArg, Ident, Pat, PatIdent, ext::IdentExt};

mod action;
pub(crate) mod choice;
mod end;
pub(crate) mod question;
mod while_loop;

/// One drawn unit: the synthetic start node, one block of the flow, or one case
/// derived from a choice. `Flow::blocks` carries the implicit end block last, so
/// end needs no variant of its own.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum NodeId {
    Start,
    Block(usize),
    Case { choice: usize, branch: usize },
}

impl NodeId {
    /// Authored position: start first, then every block followed by its own
    /// cases. A derived ordering would put every case after every block, so the
    /// order the layout visits nodes in is written out here instead.
    const fn key(self) -> (usize, usize, usize) {
        match self {
            Self::Start => (0, 0, 0),
            Self::Block(block) => (block + 1, 0, 0),
            Self::Case { choice, branch } => (choice + 1, 1, branch),
        }
    }
}

impl Ord for NodeId {
    fn cmp(&self, other: &Self) -> Ordering {
        self.key().cmp(&other.key())
    }
}

impl PartialOrd for NodeId {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// The visual role of a node, which decides its shape and its accessible name.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum NodeKind {
    Start,
    Action,
    Question,
    Select,
    Case,
    End,
}

/// One outgoing attachment point. A question is the only node with more than one
/// exit, so `branch` names the output an exit carries there; every other exit
/// leaves it unset, a select's distributor and a case's own exit included.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct ExitId {
    pub(crate) node: NodeId,
    pub(crate) branch: Option<usize>,
}

impl ExitId {
    pub(crate) const fn of(node: NodeId) -> Self {
        Self { node, branch: None }
    }
}

/// Where a connection starts: an exit or an implicit junction.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) enum Source {
    Exit(ExitId),
    Junction(usize),
}

/// Where a connection ends: a node, or the junction a producer branch enters.
/// Every destination is a vertex of the precedence graph, so it is that type.
pub(crate) type Destination = Vertex;

/// RFC 0002 §7 identifies a connection by its source exit and its destination,
/// so connections leaving distinct exits of one node stay distinct even when
/// they join the same pair of nodes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct Connection {
    pub(crate) source: Source,
    pub(crate) destination: Destination,
}

/// One node and the labels it owns. RFC 0002 §6 gives the capture list to the
/// node rather than to an incoming connection, so it is drawn once however many
/// connections arrive.
pub(crate) struct Node {
    pub(crate) id: NodeId,
    pub(crate) kind: NodeKind,
    pub(crate) label: String,
    /// The wires this node captures, in authored order, with capture modifiers.
    pub(crate) capture: Vec<String>,
}

/// One exit and the hand-over it owns: the wires newly provided here, labeled
/// whether or not any connection leaves.
pub(crate) struct Exit {
    pub(crate) id: ExitId,
    pub(crate) handover: Vec<String>,
    /// An optional authored description of a question branch.
    pub(crate) branch_description: Option<String>,
}

/// One implicit meeting point. A junction adds no block, no producer occurrence
/// and no capture. Alternative producers meet before a consumer captures what
/// they provide; loop entries and iteration tails have no merged wires.
///
/// Merged wires that the same alternatives provide, ordered behind the same
/// branch-local work, converge at the same place, so they share one junction:
/// drawing the meeting twice would repeat one convergence and force the two
/// copies to cross.
pub(crate) struct Junction {
    pub(crate) wires: Vec<String>,
}

pub(crate) struct Topology {
    pub(crate) nodes: Vec<Node>,
    pub(crate) exits: Vec<Exit>,
    pub(crate) junctions: Vec<Junction>,
    pub(crate) connections: Vec<Connection>,
    /// Every node and junction, in authored order, as the connections address
    /// them. Sorted, so a lookup is a binary search.
    pub(crate) vertices: Vec<Vertex>,
    /// Structural precedence after the iteration, never drawn as execution.
    pub(crate) order: Vec<Connection>,
    pub(crate) back_edges: Vec<Connection>,
    pub(crate) loops: Vec<Loop>,
}

impl Topology {
    pub(crate) fn node(&self, id: NodeId) -> &Node {
        self.nodes
            .iter()
            .find(|node| node.id == id)
            .expect("every drawn node is projected")
    }

    /// The connections leaving one exit, which decide whether its hand-over may
    /// share a label with the capture at the other end.
    pub(crate) fn leaving(&self, exit: ExitId) -> impl Iterator<Item = &Connection> {
        self.connections
            .iter()
            .filter(move |connection| connection.source == Source::Exit(exit))
    }

    pub(crate) fn handover(&self, exit: ExitId) -> &[String] {
        self.exit(exit).handover.as_slice()
    }

    pub(crate) fn exit(&self, exit: ExitId) -> &Exit {
        self.exits
            .iter()
            .find(|owner| owner.id == exit)
            .expect("every addressed exit is projected")
    }

    pub(crate) fn capture(&self, node: NodeId) -> &[String] {
        self.node(node).capture.as_slice()
    }

    /// The displayed capture list. `()` marks an empty input on a connected
    /// computational node; it is neither a wire name nor a synthetic capture.
    pub(crate) fn capture_label(&self, node: NodeId) -> Vec<String> {
        let projected = self.node(node);
        if projected.capture.is_empty()
            && matches!(
                projected.kind,
                NodeKind::Action | NodeKind::Question | NodeKind::Select
            )
            && self.incoming(Vertex::Node(node)).next().is_some()
        {
            vec!["()".to_owned()]
        } else {
            projected.capture.clone()
        }
    }

    pub(crate) fn incoming(&self, vertex: Vertex) -> impl Iterator<Item = &Connection> {
        self.connections
            .iter()
            .filter(move |connection| connection.destination == vertex)
    }

    pub(crate) fn outgoing(&self, vertex: Vertex) -> impl Iterator<Item = &Connection> {
        self.connections
            .iter()
            .filter(move |connection| Vertex::from(connection.source) == vertex)
    }

    /// Whether one forward connection enters the node.
    pub(crate) fn single_arrival(&self, node: NodeId) -> bool {
        self.incoming(Vertex::Node(node)).count() == 1
    }

    /// Equal, nonempty displayed lists share a label only at the sole connection
    /// between their ends. Capture modifiers and the empty-input marker never
    /// match a bare wire name, and an exit handing over nothing shares no label
    /// with a node that shows no capture list either.
    ///
    /// Only the outdegree check is reached by an authored flow, at a select
    /// distributor: alternatives meet at a junction, so no node in the current
    /// fixtures has two incoming connections. RFC 0002 §7 still allows one, and
    /// the indegree check is kept for it.
    pub(crate) fn shares_label(&self, connection: &Connection) -> bool {
        let (Source::Exit(exit), Destination::Node(node)) =
            (connection.source, connection.destination)
        else {
            return false;
        };
        // ponytail: two scans of every connection per label; keep counts beside
        // the exits if a diagram ever outgrows a few dozen connections.
        !self.handover(exit).is_empty()
            && self.handover(exit) == self.capture_label(node)
            && self.leaving(exit).count() == 1
            && self.single_arrival(node)
    }
}

/// Builds the topology of one validated flow. `start` labels the start node
/// and `return_type` captions end, both taken from the authored source text.
pub(crate) fn project(model: &SemanticModel, start: &str, return_type: &str) -> Topology {
    let mut nodes = vec![Node {
        id: NodeId::Start,
        kind: NodeKind::Start,
        label: start.to_owned(),
        capture: Vec::new(),
    }];
    let mut exits = vec![Exit {
        id: ExitId::of(NodeId::Start),
        handover: model
            .parameters
            .iter()
            .filter_map(|parameter| {
                let FnArg::Typed(parameter) = parameter else {
                    return None;
                };
                let Pat::Ident(binding) = parameter.pat.as_ref() else {
                    return None;
                };
                Some(provided(binding))
            })
            .collect(),
        branch_description: None,
    }];
    for (index, block) in model.flow.blocks.iter().enumerate() {
        match block.kind {
            BlockKind::Action => action::project(index, block, &mut nodes, &mut exits),
            BlockKind::Question => question::project(index, block, &mut nodes, &mut exits),
            BlockKind::While => while_loop::project(index, block, &mut nodes, &mut exits),
            BlockKind::Choice => choice::project(index, block, &mut nodes, &mut exits),
            BlockKind::End
                if model
                    .executions
                    .iter()
                    .any(|execution| execution.outcome == ExecutionOutcome::End) =>
            {
                end::project(index, block, return_type, &mut nodes);
            }
            BlockKind::Loop | BlockKind::End => {}
        }
    }

    let merges = merges(model);
    let loops = loops(model, merges.len());
    let loop_junctions = loops.len() * 2;
    let vertices = vertices(&nodes, merges.len() + loop_junctions);
    let connections = connections(model, &merges, &loops, &vertices);
    let mut topology = Topology {
        order: Vec::new(),
        back_edges: Vec::new(),
        loops,
        nodes,
        exits,
        connections,
        junctions: merges
            .iter()
            .map(|group| Junction {
                wires: group
                    .iter()
                    .map(|&merge| {
                        let ProducerId::BlockOutput { block, output } =
                            model.merges[merge].producers[0]
                        else {
                            unreachable!("a wire merge combines block outputs")
                        };
                        provided(model.flow.blocks[block].output_binding(output))
                    })
                    .collect(),
            })
            .chain((0..loop_junctions).map(|_| Junction { wires: Vec::new() }))
            .collect(),
        vertices,
    };
    close_loops(&mut topology);
    topology
}

/// The junctions to draw, each the merges that meet in one place, in model
/// order. A merged wire nobody captures has nothing to draw: its producers
/// still label their outputs at their own exits, and no routes meet.
fn merges(model: &SemanticModel) -> Vec<Vec<usize>> {
    let precedes = precedes(model);
    // Two merges meet in one place when the same work arrives at both. What
    // arrives is not the producer list: a producer that precedes another block
    // waiting for the same merge reaches it through that block, so only the
    // last blocks of `producers` and `before` are entered from. Once those
    // coincide the two merges are one convergence point, and the wires that
    // meet there fan out to their own consumers afterwards (RFC 0002 §7).
    let key = |merge: usize| {
        let merge = &model.merges[merge];
        // A brancher keeps the output it provides, so two merges taking
        // different cases of one choice stay apart.
        let mut entries = merge
            .before
            .iter()
            .map(|&block| (block, None))
            .collect::<BTreeMap<usize, Option<usize>>>();
        for &producer in &merge.producers {
            if let ProducerId::BlockOutput { block, output } = producer {
                let branch = branches(model.flow.blocks[block].kind);
                entries.insert(block, branch.then_some(output));
            }
        }
        entries
            .iter()
            .filter(|(block, _)| {
                !entries
                    .keys()
                    .any(|other| other != *block && precedes[**block].contains(other))
            })
            .map(|(block, output)| (*block, *output))
            .collect::<BTreeSet<_>>()
    };
    let mut groups: Vec<(_, Vec<usize>)> = Vec::new();
    for merge in 0..model.merges.len() {
        if consumers(model, &model.merges[merge].wire).next().is_none() {
            continue;
        }
        let key = key(merge);
        match groups.iter_mut().find(|(existing, _)| *existing == key) {
            Some((_, group)) => group.push(merge),
            None => groups.push((key, vec![merge])),
        }
    }
    groups.into_iter().map(|(_, group)| group).collect()
}

/// Which blocks each block precedes, over every execution's capture dependencies
/// and implicit block order. Used to tell which of a merge's antecedents actually arrive at
/// it and which reach it through another.
fn precedes(model: &SemanticModel) -> Vec<BTreeSet<usize>> {
    let blocks = model.flow.blocks.len();
    let mut later = vec![BTreeSet::new(); blocks];
    for execution in &model.executions {
        for dependency in &execution.dependencies {
            if let ProducerId::BlockOutput { block, .. } = dependency.producer {
                later[block].insert(dependency.capture.block);
            }
        }
    }
    // ponytail: a dense closure over blocks; a flow with hundreds of them would
    // want successor sets instead.
    for middle in 0..blocks {
        for block in 0..blocks {
            if later[block].contains(&middle) {
                let reachable = later[middle].clone();
                later[block].extend(reachable);
            }
        }
    }
    later
}

fn consumers<'a>(model: &'a SemanticModel, wire: &'a Ident) -> impl Iterator<Item = usize> + 'a {
    model
        .flow
        .blocks
        .iter()
        .enumerate()
        .filter(move |(_, block)| block.inputs.iter().any(|input| input.ident == *wire))
        .map(|(index, _)| index)
}

/// One exit per branch output, each handing over the single output it carries.
/// `exit` names the id the kind gives one branch, which is the only part that
/// differs between a question and a choice.
fn branch_exits(
    index: usize,
    block: &Block,
    exit: fn(usize, usize) -> ExitId,
) -> impl Iterator<Item = Exit> {
    block
        .outputs
        .iter()
        .enumerate()
        .map(move |(branch, _)| Exit {
            id: exit(index, branch),
            handover: vec![provided(block.output_binding(branch))],
            branch_description: None,
        })
}

/// Whether this kind scopes its outputs per branch, so an output is provided
/// only on the branch selected and each branch owns its own exit.
fn branches(kind: BlockKind) -> bool {
    match kind {
        BlockKind::Action | BlockKind::Loop => false,
        BlockKind::Question | BlockKind::Choice | BlockKind::While => true,
        BlockKind::End => unreachable!("end declares no output"),
    }
}

/// The node and captures common to every flow block.
fn block_node(index: usize, block: &Block, kind: NodeKind) -> Node {
    Node {
        id: NodeId::Block(index),
        kind,
        label: block.description.clone().unwrap_or_default(),
        capture: block.inputs.iter().map(captured).collect(),
    }
}

/// Capture modifiers distinguish the four authored input forms.
fn captured(input: &Input) -> String {
    let borrow = if input.borrowed { "&" } else { "" };
    let mutable = if input.mutable { "mut " } else { "" };
    format!("{borrow}{mutable}{}", input.alias.unraw())
}

fn provided(binding: &PatIdent) -> String {
    let mutable = if binding.mutability.is_some() {
        "mut "
    } else {
        ""
    };
    format!("{mutable}{}", binding.ident.unraw())
}

/// A vertex of the precedence graph. Junctions take part so that a route
/// through one suppresses the direct producer-to-consumer connection it
/// replaces; they remain absent from `nodes`, adding no block, no producer
/// occurrence and no capture.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) enum Vertex {
    Node(NodeId),
    Junction(usize),
}

impl From<Source> for Vertex {
    fn from(source: Source) -> Self {
        match source {
            Source::Exit(exit) => Self::Node(exit.node),
            Source::Junction(junction) => Self::Junction(junction),
        }
    }
}

/// The union of each execution's direct connections. Reducing every execution
/// on its own preserves a connection that is direct in one and transitively
/// redundant in another, as RFC 0002 §7 requires.
fn connections(
    model: &SemanticModel,
    merges: &[Vec<usize>],
    loops: &[Loop],
    vertices: &[Vertex],
) -> Vec<Connection> {
    let junction_of = |wire: &Ident| {
        merges
            .iter()
            .position(|group| group.iter().any(|&merge| model.merges[merge].wire == *wire))
    };

    let mut order = Vec::new();
    serial_order(&model.execution_plan, &mut order);
    let mut union = BTreeSet::new();
    for execution in &model.executions {
        let mut direct = serial_connections(model, execution, merges, loops, &order);
        for dependency in &execution.dependencies {
            let capture = dependency.capture;
            let source = source(model, dependency.producer);
            let consumer = Destination::Node(NodeId::Block(capture.block));
            let wire = &model.flow.blocks[capture.block].inputs[capture.input].ident;
            match junction_of(wire) {
                // Alternative producers meet before any capture, so the route
                // runs producer to junction to consumer rather than direct.
                Some(junction) => {
                    direct.insert(Connection {
                        source,
                        destination: Destination::Junction(junction),
                    });
                    direct.insert(Connection {
                        source: Source::Junction(junction),
                        destination: consumer,
                    });
                }
                None => {
                    direct.insert(Connection {
                        source,
                        destination: consumer,
                    });
                }
            }
        }
        // Local work in a producer branch finishes before the junction, which
        // is how a branch-local value is kept from outliving its own branch.
        for (junction, group) in merges.iter().enumerate() {
            for merge in group.iter().map(|&merge| &model.merges[merge]) {
                if !merge
                    .producers
                    .iter()
                    .any(|&producer| produced(model, execution, producer))
                {
                    continue;
                }
                direct.extend(merge.producers.iter().filter_map(|&producer| {
                    produced(model, execution, producer).then_some(Connection {
                        source: source(model, producer),
                        destination: Destination::Junction(junction),
                    })
                }));
                direct.extend(merge.after.iter().filter_map(|&block| {
                    (execution.participates(block)
                        || (model.flow.blocks[block].kind == BlockKind::End
                            && execution.outcome == ExecutionOutcome::End))
                        .then_some(Connection {
                            source: Source::Junction(junction),
                            destination: Destination::Node(NodeId::Block(block)),
                        })
                }));
                direct.extend(
                    merge
                        .before
                        .iter()
                        // Structural loops have no exit. Their participating
                        // body blocks already establish completion here.
                        .filter(|&&block| {
                            execution.participates(block)
                                && model.flow.blocks[block].kind != BlockKind::Loop
                        })
                        .map(|&block| Connection {
                            source: selected_exit(model, execution, block),
                            destination: Destination::Junction(junction),
                        }),
                );
            }
        }
        // A select precedes the case it selected.
        direct.extend(
            execution
                .blocks
                .iter()
                .filter(|&&block| model.flow.blocks[block].kind == BlockKind::Choice)
                .map(|&block| choice::connection(block, execution)),
        );

        union.extend(reduce(&direct, vertices));
    }
    union.into_iter().collect()
}

/// The selected serial route before capture dependencies and merges are added.
///
/// Where the step just taken completes a merged wire's branch-local work, the
/// route continues from that wire's junction rather than from the block's own
/// exit: the alternatives rejoin there, and one connection carries on.
/// Continuing from each producer instead would run the spine past the junction,
/// leaving the merge a parallel path beside it that no lane order can separate.
fn serial_connections(
    model: &SemanticModel,
    execution: &Execution,
    merges: &[Vec<usize>],
    loops: &[Loop],
    order: &[usize],
) -> BTreeSet<Connection> {
    let mut direct = BTreeSet::new();
    let mut previous = Source::Exit(ExitId::of(NodeId::Start));
    let mut closed = BTreeSet::new();
    let mut steps = order
        .iter()
        .copied()
        .filter(|&block| {
            execution.participates(block)
                || (model.flow.blocks[block].kind == BlockKind::End
                    && execution.outcome == ExecutionOutcome::End)
        })
        .peekable();
    while let Some(block) = steps.next() {
        for &header in execution.repeats.iter().rev() {
            if model.flow.blocks[header]
                .loop_end
                .is_some_and(|end| end <= block)
                && closed.insert(header)
            {
                let tail = loops
                    .iter()
                    .find(|loop_| loop_.header == header)
                    .expect("a repeating loop has topology")
                    .tail;
                direct.insert(Connection {
                    source: previous,
                    destination: Destination::Junction(tail),
                });
                previous = Source::Junction(tail);
            }
        }
        if let Some(loop_) = loops.iter().find(|loop_| loop_.header == block) {
            direct.insert(Connection {
                source: previous,
                destination: Destination::Junction(loop_.entry),
            });
            previous = Source::Junction(loop_.entry);
        }
        if model.flow.blocks[block].kind == BlockKind::Loop {
            continue;
        }
        direct.insert(Connection {
            source: previous,
            destination: Destination::Node(NodeId::Block(block)),
        });
        if model.flow.blocks[block].kind == BlockKind::End {
            continue;
        }
        let exit = selected_exit(model, execution, block);
        // Branch-local work still owed to the merge keeps the route on the
        // block's own exit; hopping to the junction would invert that order.
        previous = steps
            .peek()
            .and_then(|&next| junction_after(model, execution, merges, block, next))
            .map_or(exit, Source::Junction);
    }
    if matches!(execution.outcome, ExecutionOutcome::Repeat { .. }) {
        // No following block closes the final regions. Include nested while
        // tails before reaching the unconditional loop's own tail.
        for &header in execution.repeats.iter().rev() {
            if !closed.insert(header) {
                continue;
            }
            let tail = loops
                .iter()
                .find(|loop_| loop_.header == header)
                .expect("a repeating loop has topology")
                .tail;
            direct.insert(Connection {
                source: previous,
                destination: Destination::Junction(tail),
            });
            previous = Source::Junction(tail);
        }
    }
    direct
}

/// The junction the route may continue from after `block`: one this execution
/// reaches, whose last producer or branch-local block has just finished. An
/// exit feeding several junctions continues from the first, which is enough:
/// the others keep their own producer connections either way.
fn junction_after(
    model: &SemanticModel,
    execution: &Execution,
    merges: &[Vec<usize>],
    block: usize,
    next: usize,
) -> Option<usize> {
    let exit = selected_exit(model, execution, block);
    merges.iter().position(|group| {
        group
            .iter()
            .map(|&merge| &model.merges[merge])
            .any(|merge| {
                !merge.before.contains(&next)
                    && merge
                        .producers
                        .iter()
                        .any(|&producer| produced(model, execution, producer))
                    && (merge.before.contains(&block)
                        || merge
                            .producers
                            .iter()
                            .any(|&producer| source(model, producer) == exit))
            })
    })
}

/// Each body occurs once in the verified plan. Visiting branches before their
/// joins gives the source order when filtered by an execution's participants,
/// including branches yielding to an outer join.
fn serial_order(plan: &ExecutionPlan, order: &mut Vec<usize>) {
    match plan {
        ExecutionPlan::Loop { index, body } => {
            order.push(*index);
            serial_order(body, order);
        }
        ExecutionPlan::While { index, body, next } => {
            order.push(*index);
            serial_order(body, order);
            serial_order(next, order);
        }
        ExecutionPlan::Action { index, next } => {
            order.push(*index);
            serial_order(next, order);
        }
        ExecutionPlan::Question {
            index,
            branches,
            join,
        } => {
            order.push(*index);
            for branch in branches {
                serial_order(&branch.plan, order);
            }
            if let Some(join) = join {
                serial_order(&join.next, order);
            }
        }
        ExecutionPlan::Choice {
            index,
            branches,
            joins,
        } => {
            order.push(*index);
            for branch in branches {
                serial_order(&branch.plan, order);
            }
            for join in joins {
                serial_order(&join.next, order);
            }
        }
        ExecutionPlan::End { index, body, .. } => {
            serial_order(body, order);
            order.push(*index);
        }
        ExecutionPlan::EndArrival { .. }
        | ExecutionPlan::Yield { .. }
        | ExecutionPlan::Repeat { .. } => {}
    }
}

fn vertices(nodes: &[Node], junctions: usize) -> Vec<Vertex> {
    let vertices: Vec<Vertex> = nodes
        .iter()
        .map(|node| Vertex::Node(node.id))
        .chain((0..junctions).map(Vertex::Junction))
        .collect();
    // `reduce` looks a vertex up by binary search, which is unspecified rather
    // than loud on an unsorted list, so every projection must push its nodes in
    // `NodeId::key` order.
    debug_assert!(vertices.is_sorted(), "vertices must stay sorted");
    vertices
}

/// Drops every connection represented through a chain of other connections.
fn reduce(direct: &BTreeSet<Connection>, vertices: &[Vertex]) -> Vec<Connection> {
    // ponytail: a dense closure is O(vertices³) per execution; switch to
    // per-vertex successor sets if a diagram ever outgrows a few dozen nodes.
    let count = vertices.len();
    let index = |vertex: Vertex| {
        vertices
            .binary_search(&vertex)
            .expect("every connection endpoint is a vertex")
    };
    let mut precedes = vec![vec![false; count]; count];
    for connection in direct {
        precedes[index(connection.source.into())][index(connection.destination)] = true;
    }
    for middle in 0..count {
        for from in 0..count {
            for to in 0..count {
                precedes[from][to] |= precedes[from][middle] && precedes[middle][to];
            }
        }
    }

    direct
        .iter()
        .copied()
        .filter(|connection| {
            let from = index(connection.source.into());
            let to = index(connection.destination);
            !(0..count).any(|middle| precedes[from][middle] && precedes[middle][to])
        })
        .collect()
}

/// Reports whether one producer occurrence provides its wire in this execution.
/// A branch output does so only when its own branch was selected.
fn produced(model: &SemanticModel, execution: &Execution, producer: ProducerId) -> bool {
    match producer {
        ProducerId::FlowInput(_) => true,
        ProducerId::BlockOutput { block, output } => {
            execution.participates(block)
                && (!branches(model.flow.blocks[block].kind)
                    || execution.selected(block) == Some(output))
        }
    }
}

/// The exit at which one producer occurrence appears: the start node for a flow
/// input, and otherwise the exit that carries that output. A branch output only
/// ever produces on its own branch, so its position is the selected one and
/// needs no lookup.
fn source(model: &SemanticModel, producer: ProducerId) -> Source {
    match producer {
        ProducerId::FlowInput(_) => Source::Exit(ExitId::of(NodeId::Start)),
        ProducerId::BlockOutput { block, output } => exit(model, block, output),
    }
}

/// The exit the selected route leaves `block` by. A brancher that takes part
/// has recorded the branch it took; every other kind has one exit, which carries
/// every output it provides.
fn selected_exit(model: &SemanticModel, execution: &Execution, block: usize) -> Source {
    let output = if branches(model.flow.blocks[block].kind) {
        execution
            .selected(block)
            .expect("a participating brancher selects a branch")
    } else {
        0
    };
    exit(model, block, output)
}

fn exit(model: &SemanticModel, block: usize, output: usize) -> Source {
    Source::Exit(match model.flow.blocks[block].kind {
        BlockKind::Action => action::exit(block),
        BlockKind::Question | BlockKind::While => question::exit(block, output),
        BlockKind::Choice => choice::exit(block, output),
        BlockKind::End | BlockKind::Loop => unreachable!("this kind has no exit"),
    })
}

#[derive(Clone, Copy)]
pub(crate) struct Loop {
    pub(crate) header: usize,
    pub(crate) entry: usize,
    pub(crate) tail: usize,
}

fn loops(model: &SemanticModel, wire_merges: usize) -> Vec<Loop> {
    loop_headers(model)
        .into_iter()
        .enumerate()
        .map(|(offset, header)| Loop {
            header,
            tail: wire_merges + offset * 2,
            entry: wire_merges + offset * 2 + 1,
        })
        .collect()
}

/// Tail-to-continuation edges constrain placement only: execution returns to
/// the loop entry instead.
fn close_loops(topology: &mut Topology) {
    for loop_ in topology.loops.clone() {
        let source = Source::Junction(loop_.tail);
        let mut forward = Vec::new();
        for connection in std::mem::take(&mut topology.connections) {
            if connection.source == source {
                topology.order.push(connection);
            } else {
                forward.push(connection);
            }
        }
        topology.connections = forward;
        topology.back_edges.push(Connection {
            source,
            destination: Destination::Junction(loop_.entry),
        });
    }
    defer_continuations(topology);
}

/// Keep joins, iteration tails, and end below the preceding body. The blocks
/// leading to them may fill their branch columns immediately after the NO exit.
/// Walk only after every loop has separated its return from forward execution.
fn defer_continuations(topology: &mut Topology) {
    for connection in std::mem::take(&mut topology.order) {
        let mut pending = vec![connection.destination];
        let mut seen = BTreeSet::new();
        while let Some(destination) = pending.pop() {
            if !seen.insert(destination) {
                continue;
            }
            let boundary = match destination {
                Vertex::Node(node) => topology.node(node).kind == NodeKind::End,
                Vertex::Junction(junction) => {
                    !topology.loops.iter().any(|loop_| loop_.entry == junction)
                }
            };
            if boundary {
                topology.order.push(Connection {
                    source: connection.source,
                    destination,
                });
            } else {
                pending.extend(topology.outgoing(destination).map(|edge| edge.destination));
            }
        }
    }
}

/// Terminal-only bodies never reach a tail and need no return connection.
fn loop_headers(model: &SemanticModel) -> Vec<usize> {
    model
        .executions
        .iter()
        .flat_map(|execution| execution.repeats.iter().copied())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

#[cfg(test)]
mod tests;
