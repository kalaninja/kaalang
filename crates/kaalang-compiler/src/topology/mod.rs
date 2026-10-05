//! Projects analyzed flows into nodes, exits, junctions, and connections.
//! Connections combine each execution's direct precedence, using
//! the verified plan's source order. Coordinates and captions belong to rendering.

use std::{
    cmp::Ordering,
    collections::{BTreeMap, BTreeSet},
};

pub(crate) use cycle::{exhausted_hop, opens_for_cycle};
use proc_macro2::Ident;

use crate::model::{
    Block, BlockKind, CaptureDependency, Execution, ExecutionOutcome, ExecutionPlan, Flow,
    ProducerId, WireMerge,
};

mod action;
mod call;
mod choice;
mod continue_block;
mod cycle;
mod end;
mod export;
mod question;
mod stage;

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
    /// Orders start first, then each block followed by its cases.
    /// Derived enum order would put all cases after all blocks.
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
    StageEntry,
    Action,
    Call,
    Cycle,
    Question,
    Select,
    Case,
    End,
    StageTransition,
    /// The for-entry of an expanded for cycle, which takes its next item.
    ForEntry,
    /// The for-end of an expanded for cycle, where each iteration ends and
    /// from which the completed cycle continues.
    ForEnd,
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

/// A connection is identified by its source exit and destination,
/// so connections leaving distinct exits of one node stay distinct even when
/// they join the same pair of nodes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Connection {
    pub source: Source,
    pub destination: Destination,
}

/// Node identity and visual role. Captions and captures are derived from the flow.
#[derive(Clone)]
pub struct Node {
    pub id: NodeId,
    pub kind: NodeKind,
}

/// Exit and newly provided producers, including those with no outgoing connection.
#[derive(Clone)]
pub struct Exit {
    pub id: ExitId,
    /// The occurrences this exit provides, in declaration order.
    pub provides: Vec<ProducerId>,
}

/// A merge or cycle/transfer boundary, with no computation or new producer.
/// Merges with identical arrivals share one junction. A cycle result may reuse
/// the merge feeding its output; other structural junctions merge no wires.
#[derive(Clone, Default)]
pub struct Junction {
    /// The `SemanticModel::merges` entries that meet here, in model order.
    pub merges: Vec<usize>,
    pub is_cycle_entry: bool,
    pub is_cycle_result: bool,
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
    pub cycle_boundaries: Vec<CycleBoundary>,
    /// Repeating cycles only, in header order, so an enclosing cycle comes
    /// before those nested in it.
    pub cycles: Vec<Cycle>,
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
    /// Vertices drawn inside the cycle body beginning at `header`: its own
    /// blocks and their cases, together with its entry and tail and those of the
    /// cycles nested in it.
    ///
    /// # Panics
    ///
    /// Panics if `header` is not a cycle header in the corresponding flow.
    #[must_use]
    pub fn body_vertices(&self, flow: &Flow, header: usize) -> BTreeSet<Vertex> {
        let end = flow.blocks[header].cycle_end.expect("a cycle owns a body");
        let body = header + 1..end;
        let results = self
            .cycle_boundaries
            .iter()
            .find(|boundary| boundary.header == header)
            .map(|boundary| boundary.results.as_slice())
            .unwrap_or_default();
        let mut vertices = BTreeSet::new();
        for node in &self.nodes {
            let block = match node.id {
                NodeId::Block(block) => block,
                NodeId::Case { choice, .. } => choice,
                NodeId::Start => continue,
            };
            if body.contains(&block) {
                vertices.insert(Vertex::Node(node.id));
            }
        }
        for cycle in &self.cycles {
            if cycle.header == header || body.contains(&cycle.header) {
                vertices.insert(Vertex::Junction(cycle.entry));
                vertices.insert(Vertex::Junction(cycle.tail));
            }
        }
        for boundary in &self.cycle_boundaries {
            if boundary.header == header {
                vertices.insert(boundary.entry);
                vertices.extend(
                    boundary
                        .results
                        .iter()
                        .map(|&result| Vertex::from(result))
                        .filter(|result| {
                            matches!(result, Vertex::Junction(junction)
                                if !self.junctions[*junction].merges.is_empty())
                        }),
                );
            } else if body.contains(&boundary.header) {
                vertices.insert(boundary.entry);
                vertices.extend(boundary.results.iter().map(|&result| Vertex::from(result)));
            }
        }
        // A wire merge or an export inside the body draws a junction and no node, so
        // the blocks alone miss it. Everything reaching such a junction comes from
        // the body, and a chain of them needs more than one pass.
        let mut settled = false;
        while !settled {
            settled = true;
            for junction in 0..self.junctions.len() {
                let vertex = Vertex::Junction(junction);
                if vertices.contains(&vertex) {
                    continue;
                }
                // Cycle interfaces and tails belong to their own lexical cycle.
                // Those of this body were inserted above; reaching an enclosing
                // boundary or a following cycle does not make that continuation
                // part of the body.
                if self
                    .cycles
                    .iter()
                    .any(|cycle| junction == cycle.entry || junction == cycle.tail)
                    || self.junctions[junction].is_cycle_result
                {
                    continue;
                }
                let mut arrivals = self.incoming(vertex).peekable();
                if arrivals.peek().is_some()
                    && arrivals.all(|edge| {
                        !results.contains(&edge.source)
                            && vertices.contains(&Vertex::from(edge.source))
                    })
                {
                    vertices.insert(vertex);
                    settled = false;
                }
            }
        }
        vertices
    }

    /// The for-entry of the for cycle a for-end closes, in its column.
    #[must_use]
    pub fn for_entry(&self, bottom: NodeId) -> Option<NodeId> {
        self.cycle_boundaries
            .iter()
            .find_map(|boundary| boundary.caps.filter(|caps| caps.bottom == bottom))
            .map(|caps| caps.top)
    }

    /// Orders each iteration end of the cycles whose headers lie in `range`
    /// before `destination`: a repeating cycle's tail, or a for cycle's
    /// for-end.
    pub(crate) fn order_iteration_ends(
        &mut self,
        range: std::ops::Range<usize>,
        destination: Destination,
    ) {
        let tails = self
            .cycles
            .iter()
            .filter(|cycle| range.contains(&cycle.header))
            .map(|cycle| Source::Junction(cycle.tail));
        let ends = self
            .cycle_boundaries
            .iter()
            .filter(|boundary| range.contains(&boundary.header))
            .filter_map(|boundary| Some(Source::Exit(ExitId::of(boundary.caps?.bottom))));
        let sources = tails.chain(ends).collect::<Vec<_>>();
        self.order
            .extend(sources.into_iter().map(|source| Connection {
                source,
                destination,
            }));
    }

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
    /// A tail stays a tail when a merge serves as it: other arrivals still keep
    /// it below the body.
    pub(crate) fn same_row_junction(&self, connection: &Connection) -> bool {
        let (Source::Exit(exit), Destination::Junction(junction)) =
            (connection.source, connection.destination)
        else {
            return false;
        };
        if exit.branch.is_none_or(|branch| branch == 0) {
            return false;
        }
        if self.cycles.iter().any(|cycle| cycle.tail == junction) {
            return self.incoming(connection.destination).count() == 1;
        }
        self.junctions
            .get(junction)
            .is_some_and(|junction| !junction.merges.is_empty())
    }
}

/// Analysis inputs available before a `SemanticModel` has been assembled.
pub(crate) struct Analyzed<'a> {
    pub(crate) flow: &'a Flow,
    pub(crate) executions: &'a [Execution],
    pub(crate) symbolic: Option<&'a crate::symbolic::Executions>,
    pub(crate) merges: &'a [WireMerge],
    pub(crate) execution_plan: &'a ExecutionPlan,
    /// Whether each cycle draws as one collapsed node instead of its body.
    pub(crate) collapse_cycles: bool,
}

/// Builds the visual topology of one validated flow.
#[allow(clippy::too_many_lines)] // Coordinates each projection phase in dependency order.
pub(crate) fn project(model: &Analyzed<'_>) -> Topology {
    let mut nodes = vec![Node {
        id: NodeId::Start,
        kind: if matches!(model.flow.kind, crate::FlowKind::Stage { .. }) {
            NodeKind::StageEntry
        } else {
            NodeKind::Start
        },
    }];
    let mut exits = vec![Exit {
        id: ExitId::of(NodeId::Start),
        // Every named flow input appears as an output of start.
        provides: (0..model.flow.flow_inputs.len())
            .map(ProducerId::FlowInput)
            .collect(),
    }];
    for (index, block) in model.flow.blocks.iter().enumerate() {
        if model.collapse_cycles && block.parent.is_some() {
            continue;
        }
        match block.kind {
            BlockKind::Action => action::project(index, block, &mut nodes, &mut exits),
            BlockKind::Call => call::project(index, block, &mut nodes, &mut exits),
            BlockKind::Cycle if model.collapse_cycles => {
                let completes = model
                    .executions
                    .iter()
                    .any(|execution| model.flow.completes_cycle(execution, index));
                cycle::project_collapsed(index, block, completes, &mut nodes, &mut exits);
            }
            // A for cycle's hidden choice and continue draw as its for-entry
            // and for-end.
            BlockKind::Choice if model.flow.takes_next_item(index) => {
                cycle::project_for_entry(index, &mut nodes, &mut exits);
            }
            BlockKind::Continue if model.flow.ends_iteration(index) => {
                cycle::project_for_end(model.flow, index, &mut nodes, &mut exits);
            }
            BlockKind::Question => question::project(index, block, &mut nodes, &mut exits),
            BlockKind::Choice => choice::project(index, block, &mut nodes, &mut exits),
            BlockKind::Return if block.transition_target.is_some() && model.executions.iter().any(|execution| execution.participates(index)) => {
                nodes.push(block_node(index, NodeKind::StageTransition));
            }
            BlockKind::End
                if model.executions.iter().any(|execution| {
                    matches!(execution.outcome, ExecutionOutcome::Return { block_index } if model.flow.blocks[block_index].transition_target.is_none())
                }) =>
            {
                end::project(index, &mut nodes);
            }
            BlockKind::Cycle
            | BlockKind::Export
            | BlockKind::Continue
            | BlockKind::Return
            | BlockKind::End => {}
        }
    }

    let merges = merges(model);
    let mut count = merges.len();
    let (cycle_boundaries, cycles) = if model.collapse_cycles {
        (Vec::new(), Vec::new())
    } else {
        let mut found = boundaries(model, &mut count);
        let cycles = cycles(model, &mut found, &mut count);
        (found, cycles)
    };
    let structural = model
        .flow
        .blocks
        .iter()
        .enumerate()
        .filter_map(|(block, statement)| {
            if model.collapse_cycles && statement.parent.is_some() {
                return None;
            }
            let junction = match statement.kind {
                BlockKind::Cycle if model.collapse_cycles => return None,
                BlockKind::Cycle => cycle_boundaries
                    .iter()
                    .find(|boundary| boundary.header == block)
                    .and_then(|boundary| match boundary.entry {
                        Vertex::Junction(junction) => Some(junction),
                        // A for cycle is entered at its for-entry node.
                        Vertex::Node(_) => None,
                    })?,
                BlockKind::Continue if statement.inputs.is_empty() => {
                    return None;
                }
                _ => transfer_junction(model, &cycles, &cycle_boundaries, block)?,
            };
            Some((block, junction))
        })
        .collect::<BTreeMap<_, _>>();
    let vertices = vertices(&nodes, count);
    let mut connections = connections(
        model,
        &merges,
        &cycles,
        &cycle_boundaries,
        &structural,
        &vertices,
    );
    // Several iteration endings of a for cycle meet on its rail, then enter
    // the for-end together.
    connections.extend(cycle_boundaries.iter().filter_map(|boundary| {
        let caps = boundary.caps?;
        Some(Connection {
            source: Source::Junction(caps.rail?),
            destination: Destination::Node(caps.bottom),
        })
    }));
    connections.sort_unstable();
    let mut topology = Topology {
        order: Vec::new(),
        back_edges: Vec::new(),
        cycle_boundaries,
        cycles,
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
    for boundary in &topology.cycle_boundaries {
        if boundary.caps.is_some() {
            continue;
        }
        topology.junctions[boundary.entry_junction()].is_cycle_entry = true;
        for result in boundary.result_junctions() {
            topology.junctions[result].is_cycle_result = true;
        }
    }
    // A collapsed projection has no cycles, so nothing below has work there.
    if !model.collapse_cycles {
        cycle::order_exits(model, &structural, &mut topology);
        close_cycles(&mut topology);
        cycle::order_boundaries(&mut topology);
        cycle::coalesce_boundaries(&mut topology);
        cycle::precede_for_ends(model.flow, &mut topology);
    }
    end::order(&mut topology);
    stage::order(&mut topology);
    topology
}

/// The junctions to draw, each the merges that meet in one place, in model
/// order. A merged wire nobody captures has nothing to draw: its producers
/// still label their outputs at their own exits, and no routes meet.
fn merges(model: &Analyzed<'_>) -> Vec<Vec<usize>> {
    let precedes = precedes(model);
    // Merge identity follows final arrivals, not all producers: earlier work
    // reaches the junction through the last blocks in `producers` and `before`.
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
                    let branch = model.flow.blocks[block].branch_count() > 0;
                    entries.insert(block, branch.then_some(output));
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
        if !model.merges[merge]
            .after
            .iter()
            .any(|&block| represented_block(model, block))
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

/// Transitive capture successors across executions, used to find final merge arrivals.
fn precedes(model: &Analyzed<'_>) -> Vec<BTreeSet<usize>> {
    let blocks = model.flow.blocks.len();
    let mut later = vec![BTreeSet::new(); blocks];
    for execution in model.executions {
        for dependency in &execution.dependencies {
            match (entered(model, dependency), dependency.producer) {
                (Some(block), _) | (None, ProducerId::BlockOutput { block, .. }) => {
                    later[block].insert(dependency.capture.block);
                }
                (None, ProducerId::FlowInput(_)) => {}
            }
        }
    }
    crate::analyze::close(&mut later, (0..blocks).rev());
    later
}

/// One exit per branch output, each handing over the single output it carries.
/// `exit` names the id the kind gives one branch, which is the only part that
/// differs between a question and a choice.
/// One branch's exit of a node drawn once for all its branches, a question or
/// a collapsed cycle with several outputs: the branch is named in the exit
/// rather than in the node.
const fn drawn_branch_exit(index: usize, branch: usize) -> ExitId {
    ExitId {
        node: NodeId::Block(index),
        branch: Some(branch),
    }
}

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

/// The only item of `items`, when there is exactly one.
pub(crate) fn sole<T>(mut items: impl Iterator<Item = T>) -> Option<T> {
    let first = items.next()?;
    items.next().is_none().then_some(first)
}

/// Orders each of `sinks` before every one of `targets`, then normalizes the
/// order. Placement-only edges use `ExitId::of` even for nodes without that
/// exit: readers inspect only the source vertex, and these edges are never drawn.
fn order_before(
    order: &mut Vec<Connection>,
    sinks: impl IntoIterator<Item = Vertex>,
    targets: impl IntoIterator<Item = Vertex>,
) {
    let sources = sinks
        .into_iter()
        .map(|vertex| match vertex {
            Vertex::Node(node) => Source::Exit(ExitId::of(node)),
            Vertex::Junction(junction) => Source::Junction(junction),
        })
        .collect::<Vec<_>>();
    for destination in targets {
        order.extend(sources.iter().map(|&source| Connection {
            source,
            destination,
        }));
    }
    order.sort_unstable();
    order.dedup();
}

/// A nonbranching exit handing over every output in declaration order.
fn sequential_exit(index: usize, block: &Block) -> Exit {
    Exit {
        id: ExitId::of(NodeId::Block(index)),
        provides: (0..block.outputs.len())
            .map(|output| ProducerId::BlockOutput {
                block: index,
                output,
            })
            .collect(),
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

impl Source {
    /// Whether two sources are side exits of one node, which leave it along
    /// one row.
    #[must_use]
    pub fn is_side_exit_beside(self, other: Self) -> bool {
        matches!((self, other), (Self::Exit(left), Self::Exit(right))
            if left.node == right.node
                && left.branch.is_some_and(|branch| branch > 0)
                && right.branch.is_some_and(|branch| branch > 0))
    }
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
/// redundant in another.
#[allow(clippy::too_many_lines)] // Keeps one execution's connection union visibly in one pass.
fn connections(
    model: &Analyzed<'_>,
    merges: &[Vec<usize>],
    cycles: &[Cycle],
    boundaries: &[CycleBoundary],
    structural: &BTreeMap<usize, usize>,
    vertices: &[Vertex],
) -> Vec<Connection> {
    if let Some(symbolic) = model.symbolic {
        return symbolic
            .clone()
            .connections(model, merges, cycles, boundaries, structural, vertices);
    }
    let junction_of = |wire: &Ident| {
        merges
            .iter()
            .position(|group| group.iter().any(|&merge| model.merges[merge].wire == *wire))
    };

    let order = serial_blocks(model, structural);
    let mut union = BTreeSet::new();
    for execution in model.executions {
        let mut hops = BTreeSet::new();
        let mut direct = serial_connections(
            model, execution, merges, cycles, boundaries, structural, &order, &mut hops,
        );
        for dependency in &execution.dependencies {
            let capture = dependency.capture;
            let returns = model.flow.blocks[capture.block].kind == BlockKind::Return;
            if !returns && !represented(model, structural, capture.block) {
                continue;
            }
            let entered = entered(model, dependency);
            let source = entered.map_or_else(
                || source(model, dependency.producer, boundaries),
                |header| entry_source(structural, header),
            );
            let consumer = if returns {
                model.flow.blocks[capture.block]
                    .transition_target
                    .map_or_else(
                        || end::destination(model.flow),
                        |_| Destination::Node(NodeId::Block(capture.block)),
                    )
            } else {
                destination(model, structural, capture.block)
            };
            let wire = &model.flow.blocks[capture.block].inputs[capture.input].ident;
            // A wire entering a cycle has already met its merge outside it.
            match junction_of(wire).filter(|_| entered.is_none()) {
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
                    .any(|&producer| model.flow.produces(execution, producer))
                {
                    continue;
                }
                direct.extend(merge.producers.iter().filter_map(|&producer| {
                    model
                        .flow
                        .produces(execution, producer)
                        .then_some(Connection {
                            source: source(model, producer, boundaries),
                            destination: Destination::Junction(junction),
                        })
                }));
                // Serial routes carry the merge through omitted structural
                // consumers to the next visible block or iteration tail.
                direct.extend(
                    merge
                        .after
                        .iter()
                        .filter(|&&block| {
                            represented(model, structural, block)
                                && visits(model.flow, execution, block)
                        })
                        .map(|&block| Connection {
                            source: Source::Junction(junction),
                            destination: destination(model, structural, block),
                        }),
                );
                direct.extend(
                    merge
                        .before
                        .iter()
                        .filter(|&&block| {
                            execution.participates(block) && represented(model, structural, block)
                        })
                        .map(|&block| Connection {
                            source: departure(model, execution, boundaries, structural, block),
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
                        && represented_block(model, block)
                        && !model.flow.takes_next_item(block)
                })
                .map(|&block| choice::connection(block, execution)),
        );

        union.extend(
            reduce(&direct, vertices)
                .into_iter()
                .filter(|edge| !hops.contains(edge)),
        );
    }
    union.into_iter().collect()
}

/// Whether `execution` runs `block`, counting the end when it returns from
/// the flow rather than through a stage transition.
fn visits(flow: &Flow, execution: &Execution, block: usize) -> bool {
    execution.participates(block)
        || (flow.blocks[block].kind == BlockKind::End
            && matches!(execution.outcome, ExecutionOutcome::Return { block_index }
                if flow.blocks[block_index].transition_target.is_none()))
}

/// Serial route before capture edges are added. Completed branch-local work
/// continues through its merge junction; bypassing it would create a parallel
/// route that no lane order can separate.
#[allow(clippy::too_many_arguments)] // All arguments are borrowed projection context.
fn serial_connections(
    model: &Analyzed<'_>,
    execution: &Execution,
    merges: &[Vec<usize>],
    cycles: &[Cycle],
    boundaries: &[CycleBoundary],
    structural: &BTreeMap<usize, usize>,
    order: &[usize],
    hops: &mut BTreeSet<Connection>,
) -> BTreeSet<Connection> {
    let mut direct = BTreeSet::new();
    let mut previous = Source::Exit(ExitId::of(NodeId::Start));
    let mut steps = order
        .iter()
        .copied()
        .filter(|&block| visits(model.flow, execution, block))
        .peekable();
    while let Some(block) = steps.next() {
        let transfer = if model.collapse_cycles {
            None
        } else {
            transfer_junction(model, cycles, boundaries, block)
        };
        direct.insert(Connection {
            source: previous,
            destination: match transfer {
                Some(junction) if !structural.contains_key(&block) => {
                    Destination::Junction(junction)
                }
                _ => destination(model, structural, block),
            },
        });
        if let Some(junction) = transfer {
            previous = Source::Junction(junction);
            continue;
        }
        if model.flow.blocks[block].kind == BlockKind::End
            || model.flow.blocks[block].transition_target.is_some()
            || opens_for_cycle(model, block)
        {
            continue;
        }
        if execution.selected(block) == Some(1)
            && let Some(hop) = exhausted_hop(model, block)
        {
            direct.insert(hop);
            hops.insert(hop);
        }
        let exit = departure(model, execution, boundaries, structural, block);
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
        )
        .map_or(exit, Source::Junction);
    }
    direct
}

/// The junction a structural transfer ends its route at: a boundary
/// consumer's cycle result, or a continue's iteration tail.
pub(crate) fn transfer_junction(
    model: &Analyzed<'_>,
    cycles: &[Cycle],
    boundaries: &[CycleBoundary],
    block: usize,
) -> Option<usize> {
    match model.flow.blocks[block].kind {
        BlockKind::Export => export::result(model.flow, boundaries, block),
        BlockKind::Continue => continue_block::tail(model.flow, cycles, boundaries, block),
        _ => None,
    }
}

/// First reachable merge completed by `block`, provided `next` owes it no work.
/// Other merges retain their producer edges; with no next block, this merge
/// still precedes the iteration tail.
#[allow(clippy::too_many_arguments)] // Mirrors the serial projection context plus both blocks.
fn junction_after(
    model: &Analyzed<'_>,
    execution: &Execution,
    merges: &[Vec<usize>],
    boundaries: &[CycleBoundary],
    structural: &BTreeMap<usize, usize>,
    block: usize,
    next: Option<usize>,
) -> Option<usize> {
    let exit = departure(model, execution, boundaries, structural, block);
    merges.iter().position(|group| {
        group
            .iter()
            .map(|&merge| &model.merges[merge])
            .any(|merge| {
                next.is_none_or(|next| !merge.before.contains(&next))
                    && merge
                        .producers
                        .iter()
                        .any(|&producer| model.flow.produces(execution, producer))
                    && (merge.before.contains(&block)
                        || merge
                            .producers
                            .iter()
                            .any(|&producer| source(model, producer, boundaries) == exit))
            })
    })
}

/// Captured transfers use junctions instead of computational nodes. A
/// capture-free transfer is redirected directly to its boundary.
pub(crate) fn represented(
    model: &Analyzed<'_>,
    structural: &BTreeMap<usize, usize>,
    block: usize,
) -> bool {
    if model.flow.blocks[block].kind == BlockKind::End
        && !model.executions.iter().any(|execution| {
            matches!(execution.outcome, ExecutionOutcome::Return { block_index }
                if model.flow.blocks[block_index].transition_target.is_none())
        })
    {
        return false;
    }
    if model.flow.blocks[block].transition_target.is_some() {
        return represented_block(model, block);
    }
    represented_block(model, block)
        && (!matches!(
            model.flow.blocks[block].kind,
            BlockKind::Export | BlockKind::Continue | BlockKind::Return
        ) || structural.contains_key(&block))
}

pub(crate) fn represented_block(model: &Analyzed<'_>, block: usize) -> bool {
    !model.collapse_cycles || model.flow.blocks[block].parent.is_none()
}

pub(crate) fn destination(
    model: &Analyzed<'_>,
    structural: &BTreeMap<usize, usize>,
    block: usize,
) -> Destination {
    if let Some(&junction) = structural.get(&block) {
        return Destination::Junction(junction);
    }
    let node = if opens_for_cycle(model, block) {
        block + 1
    } else {
        block
    };
    Destination::Node(NodeId::Block(node))
}

/// Where a capture entering the cycle at `header` comes from: its entry
/// junction, or the for-entry of a for cycle.
pub(crate) fn entry_source(structural: &BTreeMap<usize, usize>, header: usize) -> Source {
    structural.get(&header).map_or_else(
        || Source::Exit(ExitId::of(NodeId::Block(header + 1))),
        |&junction| Source::Junction(junction),
    )
}

/// The blocks a serial route steps through, in plan order: every represented
/// block, and the capture-free continues that end a route at an iteration
/// tail. A for cycle's completion leaves its for-end, so its export, the one
/// export outside `structural`, draws nothing.
pub(crate) fn serial_blocks(
    model: &Analyzed<'_>,
    structural: &BTreeMap<usize, usize>,
) -> Vec<usize> {
    let mut order = Vec::new();
    crate::plan::serial_order(model.execution_plan, &mut order);
    order.retain(|&block| {
        represented(model, structural, block)
            || (!model.collapse_cycles
                && model.flow.blocks[block].kind == BlockKind::Continue
                && !structural.contains_key(&block))
    });
    order
}

fn departure(
    model: &Analyzed<'_>,
    execution: &Execution,
    boundaries: &[CycleBoundary],
    structural: &BTreeMap<usize, usize>,
    block: usize,
) -> Source {
    structural.get(&block).map_or_else(
        || selected_exit(model, execution, boundaries, block),
        |&junction| Source::Junction(junction),
    )
}

fn vertices(nodes: &[Node], junctions: usize) -> Vec<Vertex> {
    let vertices: Vec<Vertex> = nodes
        .iter()
        .map(|node| Vertex::Node(node.id))
        .chain((0..junctions).map(Vertex::Junction))
        .collect();
    // `reduce` uses binary search and requires `NodeId::key` order.
    debug_assert!(vertices.is_sorted(), "vertices must stay sorted");
    vertices
}

/// Transitive reduction using successor bitsets built deepest first.
/// Removes an edge only when a path of two or more edges reaches its destination.
fn reduce(direct: &BTreeSet<Connection>, vertices: &[Vertex]) -> Vec<Connection> {
    let count = vertices.len();
    let index = |vertex: Vertex| {
        vertices
            .binary_search(&vertex)
            .expect("every connection endpoint is a vertex")
    };
    let words = count.div_ceil(u64::BITS as usize);
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
            reach[vertex][successor / u64::BITS as usize] |= 1 << (successor % u64::BITS as usize);
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
            !successors[from].iter().any(|&middle| {
                middle != to
                    && reach[middle][to / u64::BITS as usize] & (1 << (to % u64::BITS as usize))
                        != 0
            })
        })
        .collect()
}

/// The cycle an outer wire enters to reach this consumer: its innermost
/// enclosing cycle, unless the wire is local to that same body. Outer data
/// arrives through that cycle's entry.
pub(crate) fn entered(model: &Analyzed<'_>, dependency: &CaptureDependency) -> Option<usize> {
    let parent = model.flow.blocks[dependency.capture.block].parent;
    parent.filter(|_| parent != model.flow.producer_cycle(dependency.producer))
}

/// Visual source of a producer: start, a cycle boundary, or a block's output exit.
pub(crate) fn source(
    model: &Analyzed<'_>,
    producer: ProducerId,
    boundaries: &[CycleBoundary],
) -> Source {
    match producer {
        ProducerId::FlowInput(_) => Source::Exit(ExitId::of(NodeId::Start)),
        ProducerId::BlockOutput { block, output }
            if model.flow.blocks[block].kind == BlockKind::Cycle && !model.collapse_cycles =>
        {
            boundaries
                .iter()
                .find(|boundary| boundary.header == block)
                .and_then(|boundary| boundary.results.get(output).copied())
                .expect("a produced cycle result has a boundary")
        }
        ProducerId::BlockOutput { block, output } => exit(model, block, output),
    }
}

/// Source of the selected branch output, or the block's sole output exit.
fn selected_exit(
    model: &Analyzed<'_>,
    execution: &Execution,
    boundaries: &[CycleBoundary],
    block: usize,
) -> Source {
    // A cycle that repeats or diverges in this execution selects nothing, and
    // its route ends inside it.
    let output = if model.flow.blocks[block].branch_count() > 0 {
        execution
            .selected(block)
            .or((model.flow.blocks[block].kind == BlockKind::Cycle).then_some(0))
            .expect("a participating brancher selects a branch")
    } else {
        0
    };
    source(model, ProducerId::BlockOutput { block, output }, boundaries)
}

pub(crate) fn exit(model: &Analyzed<'_>, block: usize, output: usize) -> Source {
    Source::Exit(match model.flow.blocks[block].kind {
        BlockKind::Question => drawn_branch_exit(block, output),
        BlockKind::Choice if model.flow.takes_next_item(block) => {
            cycle::cap_exit(model.flow, block, output)
        }
        BlockKind::Choice => choice::exit(block, output),
        // A cycle with several outputs hands each over at its own branch exit.
        BlockKind::Cycle if model.flow.blocks[block].branch_count() > 0 => {
            drawn_branch_exit(block, output)
        }
        // An action, a call and a completed cycle each hand over every output
        // at one non-branching exit.
        BlockKind::Action | BlockKind::Call | BlockKind::Cycle => ExitId::of(NodeId::Block(block)),
        BlockKind::End | BlockKind::Export | BlockKind::Continue | BlockKind::Return => {
            unreachable!("this kind has no exit")
        }
    })
}

/// Repeating cycle with entry and tail junctions and a preferred back-edge side.
#[derive(Clone, Copy)]
pub struct Cycle {
    pub header: usize,
    pub entry: usize,
    pub tail: usize,
    /// The preferred contour, not a requirement: choose the left
    /// side unless every repeating route takes the rightmost branch of the
    /// first selection in the body.
    pub(crate) prefer_left: bool,
}

/// The visible interface of one expanded cycle.
#[derive(Clone)]
pub struct CycleBoundary {
    pub header: usize,
    pub end: usize,
    /// A junction when routes meet here, or the first body node otherwise.
    pub entry: Vertex,
    /// Per declared output, in declaration order, the junction or body exit
    /// that supplies it, including its branch. Empty when the cycle never
    /// completes.
    pub results: Vec<Source>,
    /// A for cycle's for-entry and for-end, which stand in for the boundary. It
    /// stays undrawn while still keeping everything else out of the body.
    pub caps: Option<Caps>,
}

/// The two caps an expanded for cycle draws instead of a boundary.
#[derive(Clone, Copy)]
pub struct Caps {
    pub top: NodeId,
    pub bottom: NodeId,
    /// The junction several iteration endings meet on before entering the
    /// for-end. A merge may serve as it, and coalescing folds a sole ending
    /// into the for-end.
    pub(crate) rail: Option<usize>,
}

impl CycleBoundary {
    /// Projection allocates entry junctions before removing redundant ones.
    fn entry_junction(&self) -> usize {
        let Vertex::Junction(junction) = self.entry else {
            unreachable!("entry junctions are read before boundary coalescing");
        };
        junction
    }

    fn result_junctions(&self) -> impl Iterator<Item = usize> + '_ {
        self.results.iter().map(|result| {
            let Source::Junction(junction) = *result else {
                unreachable!("result junctions are read before boundary coalescing");
            };
            junction
        })
    }
}

fn boundaries(model: &Analyzed<'_>, count: &mut usize) -> Vec<CycleBoundary> {
    model
        .flow
        .blocks
        .iter()
        .enumerate()
        .filter(|(_, block)| block.kind == BlockKind::Cycle)
        .map(|(header, _)| {
            let end = model.flow.blocks[header]
                .cycle_end
                .expect("a cycle owns a body");
            // Every declared output reaches its consumer on some route, or
            // reachability already rejected the flow.
            let completes = model
                .executions
                .iter()
                .any(|execution| model.flow.completes_cycle(execution, header));
            // A for cycle opens at its for-entry and continues from its
            // for-end; neither needs a junction.
            if let Some((top, bottom)) = model.flow.for_caps(header) {
                let (top, bottom) = (NodeId::Block(top), NodeId::Block(bottom));
                return CycleBoundary {
                    header,
                    end,
                    entry: Vertex::Node(top),
                    results: completes
                        .then(|| Source::Exit(ExitId::of(bottom)))
                        .into_iter()
                        .collect(),
                    caps: Some(Caps {
                        top,
                        bottom,
                        rail: None,
                    }),
                };
            }
            let entry = *count;
            *count += 1;
            let outputs = if completes {
                model.flow.blocks[header].outputs.len()
            } else {
                0
            };
            let results = (*count..*count + outputs).map(Source::Junction).collect();
            *count += outputs;
            CycleBoundary {
                header,
                end,
                entry: Vertex::Junction(entry),
                results,
                caps: None,
            }
        })
        .collect()
}

/// Allocates where each repeating cycle ends an iteration: a loop cycle's
/// tail, or the rail a for cycle's endings meet on before its for-end.
fn cycles(model: &Analyzed<'_>, boundaries: &mut [CycleBoundary], count: &mut usize) -> Vec<Cycle> {
    let repeating = model
        .executions
        .iter()
        .filter_map(|execution| match execution.outcome {
            ExecutionOutcome::Repeat { cycle_index } => Some(cycle_index),
            ExecutionOutcome::Return { .. } => None,
        })
        .collect::<BTreeSet<_>>();
    let mut cycles = Vec::new();
    for header in repeating {
        let tail = *count;
        *count += 1;
        let boundary = boundaries
            .iter_mut()
            .find(|boundary| boundary.header == header)
            .expect("every expanded cycle has a boundary");
        if let Some(caps) = &mut boundary.caps {
            caps.rail = Some(tail);
        } else {
            cycles.push(Cycle {
                header,
                tail,
                entry: boundary.entry_junction(),
                prefer_left: cycle::prefer_left(model, header),
            });
        }
    }
    cycles
}

/// Tail-to-continuation edges constrain placement only: the iteration back edge
/// reaches the cycle entry instead.
fn close_cycles(topology: &mut Topology) {
    for cycle in topology.cycles.clone() {
        let source = Source::Junction(cycle.tail);
        topology.order.extend(
            topology
                .connections
                .iter()
                .filter(|connection| connection.source == source)
                .copied(),
        );
        topology.connections.retain(|it| it.source != source);
        topology.index();
        topology.back_edges.push(Connection {
            source,
            destination: Destination::Junction(cycle.entry),
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
                        || topology.cycles.iter().any(|cycle| cycle.tail == junction)
                        || topology.cycle_boundaries.iter().any(|boundary| {
                            boundary
                                .caps
                                .is_some_and(|caps| caps.rail == Some(junction))
                        })
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

#[cfg(test)]
mod tests;
