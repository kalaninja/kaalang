//! Projects the analyzed flow into the diagram's visual topology: its nodes,
//! exits, implicit junctions and connections, before any coordinate exists.
//!
//! RFC 0002 §7 defines the drawn connections as the union, over every possible
//! execution, of the direct precedence between participating nodes. This module
//! preserves the flow's dependencies and merges, reading its verified execution
//! plan for the same source order codegen emits.
//!
//! Only structure lives here. A node owns its role, an exit owns the producer
//! occurrences it provides, and a junction owns the merges that meet in it; the
//! displayed strings are a presentation choice a consumer derives from the
//! authored flow (RFC 0002 §6).

use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet};

use proc_macro2::Ident;

use crate::model::{
    Block, BlockKind, Execution, ExecutionOutcome, ExecutionPlan, Flow, ProducerId, WireMerge,
};

mod action;
mod break_block;
pub(crate) mod choice;
mod end;
mod loop_block;
pub(crate) mod question;

/// One drawn unit: the synthetic start node, one block of the flow, or one case
/// derived from a choice. `Flow::blocks` carries the implicit end block last, so
/// end needs no variant of its own.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum NodeId {
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
pub enum NodeKind {
    Start,
    Action,
    Loop,
    Question,
    Select,
    Case,
    End,
}

/// One outgoing attachment point. A question is the only node with more than one
/// exit, so `branch` names the output an exit carries there; every other exit
/// leaves it unset, a select's distributor and a case's own exit included.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ExitId {
    pub node: NodeId,
    pub branch: Option<usize>,
}

impl ExitId {
    #[must_use]
    pub const fn of(node: NodeId) -> Self {
        Self { node, branch: None }
    }
}

/// Where a connection starts: an exit or an implicit junction.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Source {
    Exit(ExitId),
    Junction(usize),
}

/// Where a connection ends: a node, or the junction a producer branch enters.
/// Every destination is a vertex of the precedence graph, so it is that type.
pub type Destination = Vertex;

/// RFC 0002 §7 identifies a connection by its source exit and its destination,
/// so connections leaving distinct exits of one node stay distinct even when
/// they join the same pair of nodes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Connection {
    pub source: Source,
    pub destination: Destination,
}

/// One node and the role that decides its shape. RFC 0002 §6 gives the capture
/// list to the node rather than to an incoming connection, but the displayed
/// list is derived from the authored block, not stored here.
#[derive(Clone)]
pub struct Node {
    pub id: NodeId,
    pub kind: NodeKind,
}

/// One exit and the hand-over it owns: the producer occurrences newly provided
/// here, whether or not any connection leaves. A consumer displays them as
/// RFC 0002 §6 requires.
#[derive(Clone)]
pub struct Exit {
    pub id: ExitId,
    /// The occurrences this exit provides, in declaration order.
    pub provides: Vec<ProducerId>,
}

/// A wire merge, cycle entry, iteration tail, break, or cycle result. Junctions
/// add no computational node or producer occurrence. A cycle result may share
/// the wire merge that feeds its break; other structural junctions merge no wire.
///
/// Merged wires that the same alternatives provide, ordered behind the same
/// branch-local work, converge at the same place, so they share one junction:
/// drawing the meeting twice would repeat one convergence and force the two
/// copies to cross.
#[derive(Clone, Default)]
pub struct Junction {
    /// The `SemanticModel::merges` entries that meet here, in model order.
    pub merges: Vec<usize>,
    pub is_break: bool,
    pub is_loop_entry: bool,
    pub is_loop_result: bool,
}

/// The visual topology of one flow: every drawn vertex, the connections between
/// them, the placement-only precedence they carry, and the cycles' back edges.
#[derive(Clone, Default)]
pub struct Topology {
    pub nodes: Vec<Node>,
    pub exits: Vec<Exit>,
    pub junctions: Vec<Junction>,
    pub connections: Vec<Connection>,
    /// Every node and junction, in authored order, as the connections address
    /// them. Sorted, so a lookup is a binary search.
    pub vertices: Vec<Vertex>,
    /// Placement precedence after an iteration and before end, never drawn as execution.
    pub order: Vec<Connection>,
    pub back_edges: Vec<Connection>,
    /// The visible input and result boundaries of expanded cycles.
    pub loop_boundaries: Vec<LoopBoundary>,
    pub loops: Vec<Loop>,
    /// Connection positions by destination and by source, so a neighbour
    /// lookup is an index rather than a scan of every connection.
    arrivals: BTreeMap<Vertex, Vec<usize>>,
    departures: BTreeMap<Vertex, Vec<usize>>,
}

/// A topology of block nodes joined by one connection per pair, indexed like a
/// projected one. Only tests need it: they check what happens to shapes the
/// projection never produces, such as a cycle.
#[cfg(test)]
pub(crate) fn linked(pairs: &[(usize, usize)]) -> Topology {
    let mut topology = Topology {
        connections: pairs
            .iter()
            .map(|&(from, to)| Connection {
                source: Source::Exit(ExitId::of(NodeId::Block(from))),
                destination: Destination::Node(NodeId::Block(to)),
            })
            .collect(),
        vertices: pairs
            .iter()
            .flat_map(|&(from, to)| [from, to])
            .map(|block| Vertex::Node(NodeId::Block(block)))
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect(),
        ..Topology::default()
    };
    topology.index();
    topology
}

impl Topology {
    /// The node one id addresses.
    ///
    /// # Panics
    ///
    /// Panics if the id names no projected node.
    #[must_use]
    pub fn node(&self, id: NodeId) -> &Node {
        self.nodes
            .iter()
            .find(|node| node.id == id)
            .expect("every drawn node is projected")
    }

    /// Indexes the connections by their ends. Every change to `connections`
    /// has to be followed by this, which is why the index is private.
    fn index(&mut self) {
        self.arrivals.clear();
        self.departures.clear();
        for (position, connection) in self.connections.iter().enumerate() {
            self.arrivals
                .entry(connection.destination)
                .or_default()
                .push(position);
            self.departures
                .entry(Vertex::from(connection.source))
                .or_default()
                .push(position);
        }
    }

    fn at(&self, positions: Option<&Vec<usize>>) -> impl Iterator<Item = &Connection> {
        positions
            .into_iter()
            .flatten()
            .map(|&position| &self.connections[position])
    }

    /// The connections leaving one exit, which decide whether its hand-over may
    /// share a label with the capture at the other end.
    pub fn leaving(&self, exit: ExitId) -> impl Iterator<Item = &Connection> {
        self.at(self.departures.get(&Vertex::Node(exit.node)))
            .filter(move |connection| connection.source == Source::Exit(exit))
    }

    /// The exit one id addresses.
    ///
    /// # Panics
    ///
    /// Panics if the id names no projected exit.
    #[must_use]
    pub fn exit(&self, exit: ExitId) -> &Exit {
        self.exits
            .iter()
            .find(|owner| owner.id == exit)
            .expect("every addressed exit is projected")
    }

    /// The connections arriving at one vertex.
    pub fn incoming(&self, vertex: Vertex) -> impl Iterator<Item = &Connection> {
        self.at(self.arrivals.get(&vertex))
    }

    /// The connections leaving one vertex, from any of its exits.
    pub fn outgoing(&self, vertex: Vertex) -> impl Iterator<Item = &Connection> {
        self.at(self.departures.get(&vertex))
    }

    /// Whether one forward connection enters the node.
    #[must_use]
    pub fn single_arrival(&self, node: NodeId) -> bool {
        self.incoming(Vertex::Node(node)).count() == 1
    }

    /// A side exit may meet a visible merge or its sole iteration tail on its row.
    /// A tail with no other arrivals needs no empty row before turning upward.
    pub(crate) fn same_row_junction(&self, connection: &Connection) -> bool {
        matches!(
            (connection.source, connection.destination),
            (Source::Exit(exit), Destination::Junction(junction))
                if exit.branch.is_some_and(|branch| branch > 0)
                    && (self.junctions.get(junction).is_some_and(|junction| !junction.merges.is_empty())
                        || self.loops.iter().any(|loop_| loop_.tail == junction)
                            && self.incoming(connection.destination).count() == 1)
        )
    }

    /// Whether one exit hands over exactly the wires that meet at one junction,
    /// in the same order: position for position, the occurrences it provides are
    /// alternatives of the merges that converge there.
    #[must_use]
    pub fn provides_merged_wires(
        &self,
        merges: &[WireMerge],
        exit: ExitId,
        junction: usize,
    ) -> bool {
        let group = &self.junctions[junction].merges;
        let provides = &self.exit(exit).provides;
        !group.is_empty()
            && provides.len() == group.len()
            && provides
                .iter()
                .zip(group)
                .all(|(producer, &merge)| merges[merge].producers.contains(producer))
    }
}

/// The analyzed parts the projection reads. `build` holds them before it can
/// assemble a `SemanticModel`, so the projection takes them directly rather
/// than through a model that does not exist yet.
pub(crate) struct Analyzed<'a> {
    pub(crate) flow: &'a Flow,
    pub(crate) executions: &'a [Execution],
    pub(crate) merges: &'a [WireMerge],
    pub(crate) execution_plan: &'a ExecutionPlan,
}

/// Builds the visual topology of one validated flow.
#[allow(clippy::too_many_lines)] // Coordinates each projection phase in dependency order.
pub(crate) fn project(model: &Analyzed<'_>, collapse_loops: bool) -> Topology {
    let mut nodes = vec![Node {
        id: NodeId::Start,
        kind: NodeKind::Start,
    }];
    let mut exits = vec![Exit {
        id: ExitId::of(NodeId::Start),
        // RFC 0002 §5 shows every named flow input as an output of start.
        provides: (0..model.flow.flow_inputs.len())
            .map(ProducerId::FlowInput)
            .collect(),
    }];
    for (index, block) in model.flow.blocks.iter().enumerate() {
        if collapse_loops && block.parent.is_some() {
            continue;
        }
        match block.kind {
            BlockKind::Action => action::project(index, block, &mut nodes, &mut exits),
            BlockKind::Loop if collapse_loops => {
                let completes = model
                    .executions
                    .iter()
                    .any(|execution| model.flow.completes_loop(execution, index));
                loop_block::project_collapsed(index, block, completes, &mut nodes, &mut exits);
            }
            BlockKind::Question => question::project(index, block, &mut nodes, &mut exits),
            BlockKind::Choice => choice::project(index, block, &mut nodes, &mut exits),
            BlockKind::End
                if model.executions.iter().any(|execution| {
                    matches!(execution.outcome, ExecutionOutcome::Return { .. })
                }) =>
            {
                end::project(index, &mut nodes);
            }
            BlockKind::Loop | BlockKind::Break | BlockKind::Return | BlockKind::End => {}
        }
    }

    let merges = merges(model, collapse_loops);
    let mut count = merges.len();
    let loop_boundaries = if collapse_loops {
        Vec::new()
    } else {
        boundaries(model, &mut count)
    };
    let loops = if collapse_loops {
        Vec::new()
    } else {
        loops(model, &loop_boundaries, &mut count)
    };
    let structural = model
        .flow
        .blocks
        .iter()
        .enumerate()
        .filter(|(_, block)| {
            matches!(block.kind, BlockKind::Loop | BlockKind::Break)
                && (!collapse_loops || block.parent.is_none())
        })
        .filter_map(|(block, statement)| {
            let junction = match statement.kind {
                BlockKind::Loop if collapse_loops => return None,
                BlockKind::Loop => loop_boundaries
                    .iter()
                    .find(|boundary| boundary.header == block)
                    .map(LoopBoundary::entry_junction)?,
                BlockKind::Break if statement.inputs.is_empty() => return None,
                BlockKind::Break => break_block::result(model.flow, &loop_boundaries, block),
                _ => unreachable!("only structural statements have junctions"),
            };
            Some((block, junction))
        })
        .collect::<BTreeMap<_, _>>();
    let vertices = vertices(&nodes, count);
    let connections = connections(
        model,
        &merges,
        &loops,
        &loop_boundaries,
        &structural,
        &vertices,
        collapse_loops,
    );
    let mut topology = Topology {
        order: Vec::new(),
        back_edges: Vec::new(),
        loop_boundaries,
        loops,
        nodes,
        exits,
        connections,
        junctions: merges
            .iter()
            .map(|group| Junction {
                merges: group.clone(),
                ..Junction::default()
            })
            .chain((merges.len()..count).map(|_| Junction::default()))
            .collect(),
        vertices,
        arrivals: BTreeMap::new(),
        departures: BTreeMap::new(),
    };
    topology.index();
    for (&block, &junction) in &structural {
        topology.junctions[junction].is_break = model.flow.blocks[block].kind == BlockKind::Break;
    }
    for boundary in &topology.loop_boundaries {
        topology.junctions[boundary.entry_junction()].is_loop_entry = true;
        if let Some(result) = boundary.result_junction() {
            topology.junctions[result].is_loop_result = true;
        }
    }
    if !collapse_loops {
        loop_block::order_exits(model, &structural, &mut topology);
    }
    close_loops(&mut topology);
    if !collapse_loops {
        loop_block::order_boundaries(&mut topology);
        loop_block::coalesce_boundaries(&mut topology);
    }
    end::order(&mut topology);
    topology
}

/// The junctions to draw, each the merges that meet in one place, in model
/// order. A merged wire nobody captures has nothing to draw: its producers
/// still label their outputs at their own exits, and no routes meet.
fn merges(model: &Analyzed<'_>, collapse_loops: bool) -> Vec<Vec<usize>> {
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
            match producer {
                ProducerId::BlockOutput { block, output } => {
                    let branch = branches(model.flow.blocks[block].kind);
                    entries.insert(block, branch.then_some(output));
                }
                ProducerId::CycleInput { block, .. } => {
                    entries.insert(block, None);
                }
                ProducerId::FlowInput(_) => {}
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
        if consumers(model, &model.merges[merge].wire)
            .find(|&block| represented_block(model, block, collapse_loops))
            .is_none()
        {
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
fn precedes(model: &Analyzed<'_>) -> Vec<BTreeSet<usize>> {
    let blocks = model.flow.blocks.len();
    let mut later = vec![BTreeSet::new(); blocks];
    for execution in model.executions {
        for dependency in &execution.dependencies {
            match dependency.producer {
                ProducerId::BlockOutput { block, .. } | ProducerId::CycleInput { block, .. } => {
                    later[block].insert(dependency.capture.block);
                }
                ProducerId::FlowInput(_) => {}
            }
        }
    }
    crate::analyze::close(&mut later);
    later
}

fn consumers<'a>(model: &'a Analyzed<'a>, wire: &'a Ident) -> impl Iterator<Item = usize> + 'a {
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
    (0..block.outputs.len()).map(move |branch| Exit {
        id: exit(index, branch),
        provides: vec![ProducerId::BlockOutput {
            block: index,
            output: branch,
        }],
    })
}

/// Whether this kind scopes its outputs per branch, so an output is provided
/// only on the branch selected and each branch owns its own exit.
fn branches(kind: BlockKind) -> bool {
    match kind {
        BlockKind::Action | BlockKind::Loop | BlockKind::Break | BlockKind::Return => false,
        BlockKind::Question | BlockKind::Choice => true,
        BlockKind::End => unreachable!("end declares no output"),
    }
}

/// The node common to every flow block.
const fn block_node(index: usize, kind: NodeKind) -> Node {
    Node {
        id: NodeId::Block(index),
        kind,
    }
}

/// A vertex of the precedence graph. Junctions take part so that a route
/// through one suppresses the direct producer-to-consumer connection it
/// replaces. Structural entries and exits also use junctions instead of nodes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Vertex {
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
#[allow(clippy::too_many_lines)] // Keeps one execution's connection union visibly in one pass.
fn connections(
    model: &Analyzed<'_>,
    merges: &[Vec<usize>],
    loops: &[Loop],
    boundaries: &[LoopBoundary],
    structural: &BTreeMap<usize, usize>,
    vertices: &[Vertex],
    collapse_loops: bool,
) -> Vec<Connection> {
    let junction_of = |wire: &Ident| {
        merges
            .iter()
            .position(|group| group.iter().any(|&merge| model.merges[merge].wire == *wire))
    };

    let mut order = Vec::new();
    crate::plan::serial_order(model.execution_plan, &mut order);
    order.retain(|&block| {
        represented(model, structural, block, collapse_loops)
            || (!collapse_loops
                && model.flow.blocks[block].kind == BlockKind::Break
                && !structural.contains_key(&block))
    });
    let mut union = BTreeSet::new();
    for execution in model.executions {
        let mut direct = serial_connections(
            model,
            execution,
            merges,
            loops,
            boundaries,
            structural,
            &order,
            collapse_loops,
        );
        for dependency in &execution.dependencies {
            let capture = dependency.capture;
            let returns = model.flow.blocks[capture.block].kind == BlockKind::Return;
            if !returns && !represented(model, structural, capture.block, collapse_loops) {
                continue;
            }
            let source = source(model, dependency.producer, boundaries, collapse_loops);
            let consumer = if returns {
                end::destination(model.flow)
            } else {
                destination(structural, capture.block)
            };
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
                        source: source(model, producer, boundaries, collapse_loops),
                        destination: Destination::Junction(junction),
                    })
                }));
                // Serial routes carry the merge through omitted structural
                // consumers to the next visible block or iteration tail.
                direct.extend(
                    merge
                        .after
                        .iter()
                        .filter(|&&block| represented(model, structural, block, collapse_loops))
                        .filter_map(|&block| {
                            (execution.participates(block)
                                || (model.flow.blocks[block].kind == BlockKind::End
                                    && matches!(
                                        execution.outcome,
                                        ExecutionOutcome::Return { .. }
                                    )))
                            .then_some(Connection {
                                source: Source::Junction(junction),
                                destination: destination(structural, block),
                            })
                        }),
                );
                direct.extend(
                    merge
                        .before
                        .iter()
                        .filter(|&&block| {
                            execution.participates(block)
                                && represented(model, structural, block, collapse_loops)
                        })
                        .map(|&block| Connection {
                            source: departure(
                                model,
                                execution,
                                boundaries,
                                structural,
                                block,
                                collapse_loops,
                            ),
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
                .filter(|&&block| {
                    model.flow.blocks[block].kind == BlockKind::Choice
                        && represented_block(model, block, collapse_loops)
                })
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
#[allow(clippy::too_many_arguments)] // All arguments are borrowed projection context.
fn serial_connections(
    model: &Analyzed<'_>,
    execution: &Execution,
    merges: &[Vec<usize>],
    loops: &[Loop],
    boundaries: &[LoopBoundary],
    structural: &BTreeMap<usize, usize>,
    order: &[usize],
    collapse_loops: bool,
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
                    && matches!(execution.outcome, ExecutionOutcome::Return { .. }))
        })
        .peekable();
    while let Some(block) = steps.next() {
        for &header in execution.repeats.iter().rev().filter(|_| !collapse_loops) {
            if model.flow.blocks[header]
                .loop_end
                .is_some_and(|end| end <= block)
                && closed.insert(header)
            {
                let tail = loops
                    .iter()
                    .find(|loop_| loop_.header == header)
                    .expect("a repeating cycle has topology")
                    .tail;
                direct.insert(Connection {
                    source: previous,
                    destination: Destination::Junction(tail),
                });
                previous = Source::Junction(tail);
            }
        }
        let break_result = (model.flow.blocks[block].kind == BlockKind::Break && !collapse_loops)
            .then(|| break_block::result(model.flow, boundaries, block));
        if let Some(result) = break_result
            && !structural.contains_key(&block)
        {
            direct.insert(Connection {
                source: previous,
                destination: Destination::Junction(result),
            });
            previous = Source::Junction(result);
            continue;
        }
        direct.insert(Connection {
            source: previous,
            destination: destination(structural, block),
        });
        if model.flow.blocks[block].kind == BlockKind::End {
            continue;
        }
        let exit = departure(
            model,
            execution,
            boundaries,
            structural,
            block,
            collapse_loops,
        );
        if let Some(result) = break_result {
            previous = Source::Junction(result);
            continue;
        }
        // Branch-local work still owed to the merge keeps the route on the
        // block's own exit; hopping to the junction would invert that order.
        previous = junction_after(
            model,
            execution,
            merges,
            boundaries,
            structural,
            block,
            steps.peek().copied(),
            collapse_loops,
        )
        .map_or(exit, Source::Junction);
    }
    if !collapse_loops && matches!(execution.outcome, ExecutionOutcome::Repeat { .. }) {
        // No following block connects the repeating route to its cycle tail.
        for &header in execution.repeats.iter().rev() {
            if !closed.insert(header) {
                continue;
            }
            let tail = loops
                .iter()
                .find(|loop_| loop_.header == header)
                .expect("a repeating cycle has topology")
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
/// the others keep their own producer connections either way. With no next
/// block, a completed merge still precedes the iteration tail.
#[allow(clippy::too_many_arguments)] // Mirrors the serial projection context plus both blocks.
fn junction_after(
    model: &Analyzed<'_>,
    execution: &Execution,
    merges: &[Vec<usize>],
    boundaries: &[LoopBoundary],
    structural: &BTreeMap<usize, usize>,
    block: usize,
    next: Option<usize>,
    collapse_loops: bool,
) -> Option<usize> {
    let exit = departure(
        model,
        execution,
        boundaries,
        structural,
        block,
        collapse_loops,
    );
    merges.iter().position(|group| {
        group
            .iter()
            .map(|&merge| &model.merges[merge])
            .any(|merge| {
                next.is_none_or(|next| !merge.before.contains(&next))
                    && merge
                        .producers
                        .iter()
                        .any(|&producer| produced(model, execution, producer))
                    && (merge.before.contains(&block)
                        || merge.producers.iter().any(|&producer| {
                            source(model, producer, boundaries, collapse_loops) == exit
                        }))
            })
    })
}

/// Captured transfers use junctions instead of computational nodes. A
/// capture-free transfer is redirected directly to its boundary.
fn represented(
    model: &Analyzed<'_>,
    structural: &BTreeMap<usize, usize>,
    block: usize,
    collapse_loops: bool,
) -> bool {
    represented_block(model, block, collapse_loops)
        && (!matches!(
            model.flow.blocks[block].kind,
            BlockKind::Break | BlockKind::Return
        ) || structural.contains_key(&block))
}

fn represented_block(model: &Analyzed<'_>, block: usize, collapse_loops: bool) -> bool {
    !collapse_loops || model.flow.blocks[block].parent.is_none()
}

fn destination(structural: &BTreeMap<usize, usize>, block: usize) -> Destination {
    structural
        .get(&block)
        .map_or(Destination::Node(NodeId::Block(block)), |&junction| {
            Destination::Junction(junction)
        })
}

fn departure(
    model: &Analyzed<'_>,
    execution: &Execution,
    boundaries: &[LoopBoundary],
    structural: &BTreeMap<usize, usize>,
    block: usize,
    collapse_loops: bool,
) -> Source {
    structural.get(&block).map_or_else(
        || selected_exit(model, execution, boundaries, block, collapse_loops),
        |&junction| Source::Junction(junction),
    )
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
///
/// One connection is redundant exactly when a chain of two or more reaches its
/// destination from its source, so the successors of each vertex are collected
/// once, deepest first, as bit rows. Every vertex is visited once and every
/// union costs one word per sixty-four vertices.
fn reduce(direct: &BTreeSet<Connection>, vertices: &[Vertex]) -> Vec<Connection> {
    let count = vertices.len();
    let index = |vertex: Vertex| {
        vertices
            .binary_search(&vertex)
            .expect("every connection endpoint is a vertex")
    };
    let words = count.div_ceil(64);
    let mut successors = vec![Vec::new(); count];
    for connection in direct {
        successors[index(connection.source.into())].push(index(connection.destination));
    }

    // Deepest first, so a successor's own reach is complete before it is used.
    let mut order = Vec::with_capacity(count);
    let mut state = vec![0u8; count];
    for root in 0..count {
        if state[root] != 0 {
            continue;
        }
        let mut stack = vec![(root, 0usize)];
        state[root] = 1;
        while let Some(&mut (vertex, ref mut next)) = stack.last_mut() {
            if *next < successors[vertex].len() {
                let successor = successors[vertex][*next];
                *next += 1;
                if state[successor] == 0 {
                    state[successor] = 1;
                    stack.push((successor, 0));
                }
            } else {
                state[vertex] = 2;
                order.push(vertex);
                stack.pop();
            }
        }
    }

    let mut reach = vec![vec![0u64; words]; count];
    for &vertex in &order {
        for &successor in &successors[vertex] {
            reach[vertex][successor / 64] |= 1 << (successor % 64);
            let (row, of) = reach.split_at_mut(vertex.max(successor));
            let (row, of) = if vertex < successor {
                (&mut row[vertex], &of[0])
            } else {
                (&mut of[0], &row[successor])
            };
            for (word, bits) in row.iter_mut().zip(of) {
                *word |= *bits;
            }
        }
    }

    direct
        .iter()
        .copied()
        .filter(|connection| {
            let from = index(connection.source.into());
            let to = index(connection.destination);
            !successors[from]
                .iter()
                .any(|&middle| middle != to && reach[middle][to / 64] & (1 << (to % 64)) != 0)
        })
        .collect()
}

/// Reports whether one producer occurrence provides its wire in this execution.
/// A branch output does so only when its own branch was selected.
fn produced(model: &Analyzed<'_>, execution: &Execution, producer: ProducerId) -> bool {
    model.flow.produces(execution, producer)
}

/// The exit at which one producer occurrence appears: the start node for a flow
/// input, and otherwise the exit that carries that output. A branch output only
/// ever produces on its own branch, so its position is the selected one and
/// needs no lookup.
fn source(
    model: &Analyzed<'_>,
    producer: ProducerId,
    boundaries: &[LoopBoundary],
    collapse_loops: bool,
) -> Source {
    match producer {
        ProducerId::FlowInput(_) => Source::Exit(ExitId::of(NodeId::Start)),
        ProducerId::CycleInput { block, .. } => Source::Junction(
            boundaries
                .iter()
                .find(|boundary| boundary.header == block)
                .expect("an expanded cycle input has an entry boundary")
                .entry_junction(),
        ),
        ProducerId::BlockOutput { block, output: _ }
            if model.flow.blocks[block].kind == BlockKind::Loop && !collapse_loops =>
        {
            Source::Junction(
                boundaries
                    .iter()
                    .find(|boundary| boundary.header == block)
                    .and_then(LoopBoundary::result_junction)
                    .expect("a produced cycle result has a boundary"),
            )
        }
        ProducerId::BlockOutput { block, output } => exit(model, block, output),
    }
}

/// The exit the selected route leaves `block` by. A brancher that takes part
/// has recorded the branch it took; every other kind has one exit, which carries
/// every output it provides.
fn selected_exit(
    model: &Analyzed<'_>,
    execution: &Execution,
    boundaries: &[LoopBoundary],
    block: usize,
    collapse_loops: bool,
) -> Source {
    let output = if branches(model.flow.blocks[block].kind) {
        execution
            .selected(block)
            .expect("a participating brancher selects a branch")
    } else {
        0
    };
    source(
        model,
        ProducerId::BlockOutput { block, output },
        boundaries,
        collapse_loops,
    )
}

fn exit(model: &Analyzed<'_>, block: usize, output: usize) -> Source {
    Source::Exit(match model.flow.blocks[block].kind {
        BlockKind::Action => action::exit(block),
        BlockKind::Question => question::exit(block, output),
        BlockKind::Choice => choice::exit(block, output),
        BlockKind::Loop => ExitId::of(NodeId::Block(block)),
        BlockKind::End | BlockKind::Break | BlockKind::Return => {
            unreachable!("this kind has no exit")
        }
    })
}

/// One cycle that repeats: its header block, the entry junction every arrival
/// reaches, the iteration tail its repeating routes meet at, and the contour
/// RFC 0002 §8 prefers for its iteration back edge.
#[derive(Clone, Copy)]
pub struct Loop {
    pub header: usize,
    pub entry: usize,
    pub tail: usize,
    /// The preferred contour, not a requirement: RFC 0002 §8 prefers the left
    /// side unless every repeating route takes the rightmost branch of the
    /// first selection in the body.
    pub prefer_left: bool,
}

/// The visible interface of one expanded cycle.
#[derive(Clone, Copy)]
pub struct LoopBoundary {
    pub header: usize,
    pub end: usize,
    /// A junction when routes meet here, or the first body node otherwise.
    pub entry: Vertex,
    /// A junction or the body exit supplying the result, including its branch.
    /// Absent when the cycle never completes.
    pub result: Option<Source>,
}

impl LoopBoundary {
    /// Projection allocates entry junctions before removing redundant ones.
    fn entry_junction(&self) -> usize {
        let Vertex::Junction(junction) = self.entry else {
            unreachable!("entry junctions are read before boundary coalescing");
        };
        junction
    }

    fn result_junction(&self) -> Option<usize> {
        self.result.map(|result| {
            let Source::Junction(junction) = result else {
                unreachable!("result junctions are read before boundary coalescing");
            };
            junction
        })
    }
}

fn boundaries(model: &Analyzed<'_>, count: &mut usize) -> Vec<LoopBoundary> {
    model
        .flow
        .blocks
        .iter()
        .enumerate()
        .filter(|(_, block)| block.kind == BlockKind::Loop)
        .map(|(header, _)| {
            let entry = *count;
            *count += 1;
            let result = model
                .executions
                .iter()
                .any(|execution| model.flow.completes_loop(execution, header))
                .then(|| {
                    let result = *count;
                    *count += 1;
                    result
                });
            LoopBoundary {
                header,
                end: model.flow.blocks[header]
                    .loop_end
                    .expect("a cycle owns a body"),
                entry: Vertex::Junction(entry),
                result: result.map(Source::Junction),
            }
        })
        .collect()
}

fn loops(model: &Analyzed<'_>, boundaries: &[LoopBoundary], count: &mut usize) -> Vec<Loop> {
    loop_headers(model)
        .into_iter()
        .map(|header| {
            let tail = *count;
            *count += 1;
            Loop {
                header,
                tail,
                entry: boundaries
                    .iter()
                    .find(|boundary| boundary.header == header)
                    .expect("every expanded cycle has a boundary")
                    .entry_junction(),
                prefer_left: loop_block::prefer_left(model, header),
            }
        })
        .collect()
}

/// Tail-to-continuation edges constrain placement only: the iteration back edge
/// reaches the cycle entry instead.
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
        topology.index();
        topology.back_edges.push(Connection {
            source,
            destination: Destination::Junction(loop_.entry),
        });
    }
    defer_continuations(topology);
}

/// Keep joins, iteration tails, and end below the preceding body. The blocks
/// leading to them may fill the exiting branch's column beside the cycle body.
/// Walk only after every cycle has separated its back edge from forward execution.
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
                    !topology.junctions[junction].merges.is_empty()
                        || topology.loops.iter().any(|loop_| loop_.tail == junction)
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

/// Completing-only bodies never reach a tail and need no iteration back edge.
fn loop_headers(model: &Analyzed<'_>) -> Vec<usize> {
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
