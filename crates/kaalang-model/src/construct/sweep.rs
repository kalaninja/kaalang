//! Complete sweep of vertex and shared-route exchange events. Connections and
//! back edges may change columns between events; vertices, ports and back edge
//! envelopes supply the persistent horizontal constraints. See RFC 0003 §2.1 for the finite space,
//! the strip-routing construction, and the state-equivalence argument.

use std::collections::{BTreeMap, BTreeSet};

use crate::model::{Flow, WireMerge};
use crate::topology::{ExitId, NodeId, Source, Topology, Vertex};

use super::{Arrangement, Contour, Obstruction, Route, Run, Side};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Lifeline {
    Wire(usize),
    BackEdge(usize),
}

#[derive(Clone)]
struct Step {
    vertex: Option<usize>,
    position: usize,
    consumed: Vec<Lifeline>,
    emitted: Vec<Lifeline>,
    side: Option<Side>,
}

/// Transitive closure of weak (1) and strict (2) column inequalities.
/// A positive cycle is inconsistent; a cycle of weak inequalities is equality.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
struct Order {
    size: usize,
    relation: Vec<u8>,
}

impl Order {
    fn new(size: usize) -> Self {
        Self {
            size,
            relation: vec![0; size * size],
        }
    }

    fn at(&self, left: usize, right: usize) -> u8 {
        if left == right {
            1
        } else {
            self.relation[left * self.size + right]
        }
    }

    fn insert(&mut self, left: usize, right: usize, strict: bool) -> bool {
        let value = if strict { 2 } else { 1 };
        if self.at(left, right) >= value {
            return true;
        }
        if self.at(right, left) > 0 && (strict || self.at(right, left) == 2) {
            return false;
        }
        let before = (0..self.size)
            .filter_map(|i| {
                let v = self.at(i, left);
                (v > 0).then_some((i, v))
            })
            .collect::<Vec<_>>();
        let after = (0..self.size)
            .filter_map(|i| {
                let v = self.at(right, i);
                (v > 0).then_some((i, v))
            })
            .collect::<Vec<_>>();
        for (i, a) in before {
            for &(j, b) in &after {
                let slot = &mut self.relation[i * self.size + j];
                *slot = (*slot).max(a.max(value).max(b));
            }
        }
        true
    }

    fn equal(&mut self, a: usize, b: usize) -> bool {
        self.insert(a, b, false) && self.insert(b, a, false)
    }

    /// The least nonnegative solution. Every strict edge adds one; all
    /// inequalities are already transitively closed, so strict-predecessor
    /// classes can be ranked without another search.
    fn number(&self) -> Vec<i32> {
        let mut values = vec![0; self.size];
        for _ in 0..self.size {
            let mut changed = false;
            for i in 0..self.size {
                for j in 0..self.size {
                    let edge = self.at(i, j);
                    if edge == 0 {
                        continue;
                    }
                    let wanted = values[i] + i32::from(edge == 2);
                    if wanted > values[j] {
                        values[j] = wanted;
                        changed = true;
                    }
                }
            }
            if !changed {
                return values;
            }
        }
        panic!("an acyclic column order has a finite numbering")
    }
}

/// Static column identities and constraints, derived before any event order.
struct Columns {
    vertex: Vec<usize>,
    exits: BTreeMap<ExitId, usize>,
    back_edges: Vec<usize>,
    back_edge_right: Vec<usize>,
    base: Order,
    /// An entry equals the minimum of its first branch's approach columns.
    minima: Vec<(usize, Vec<usize>)>,
    bodies: Vec<[Vec<usize>; 2]>,
}

fn root(parent: &[usize], mut i: usize) -> usize {
    while parent[i] != i {
        i = parent[i];
    }
    i
}

fn unite(parent: &mut [usize], a: usize, b: usize) {
    let a = root(parent, a);
    let b = root(parent, b);
    parent[b] = a;
}

fn index_of(topology: &Topology, vertex: Vertex) -> usize {
    topology
        .vertices
        .binary_search(&vertex)
        .expect("a projected vertex")
}

fn serial_identities(
    topology: &Topology,
    raw_exits: &BTreeMap<ExitId, usize>,
    parent: &mut [usize],
) {
    let index = |v| index_of(topology, v);
    for exit in &topology.exits {
        if exit.id.branch.is_none_or(|branch| branch == 0) {
            unite(
                parent,
                index(Vertex::Node(exit.id.node)),
                raw_exits[&exit.id],
            );
        }
    }
    // Every serial arrival shares its predecessor's current column.
    for &vertex in &topology.vertices {
        if let Some(wire) = super::serial_arrival(topology, vertex) {
            let source = match wire.source {
                Source::Exit(exit) => raw_exits[&exit],
                Source::Junction(j) => index(Vertex::Junction(j)),
            };
            unite(parent, index(vertex), source);
        }
    }
}

type Minimum = (usize, Vec<usize>);

fn branch_rules(
    flow: &Flow,
    topology: &Topology,
    raw_exits: &BTreeMap<ExitId, usize>,
    parent: &mut [usize],
) -> (Vec<(usize, usize)>, Vec<Minimum>) {
    let index = |v| index_of(topology, v);
    let reachable = super::regions::reachable(topology);
    let mut inequalities = Vec::new();
    let mut minima = Vec::new();
    for block in super::regions::branchers(flow, topology) {
        let regions = super::regions::regions(flow, topology, &reachable, block);
        let starts = (0..flow.blocks[block].branch_count())
            .map(|branch| {
                if flow.blocks[block].kind == crate::model::BlockKind::Choice {
                    index(Vertex::Node(NodeId::Case {
                        choice: block,
                        branch,
                    }))
                } else {
                    raw_exits[&ExitId {
                        node: NodeId::Block(block),
                        branch: Some(branch),
                    }]
                }
            })
            .collect::<Vec<_>>();
        unite(parent, index(Vertex::Node(NodeId::Block(block))), starts[0]);
        inequalities.extend(starts.windows(2).map(|pair| (pair[0], pair[1])));
        for (entry, first) in super::regions::continuations(topology, block, &regions.branches) {
            let mut pending = vec![entry];
            let mut approaches = BTreeSet::new();
            while let Some(vertex) = pending.pop() {
                for wire in topology.incoming(vertex) {
                    match wire.source {
                        Source::Junction(j) => pending.push(Vertex::Junction(j)),
                        Source::Exit(exit)
                            if regions.branches[first].contains(&Vertex::Node(exit.node))
                                || (exit.node == NodeId::Block(block)
                                    && exit.branch == Some(first)) =>
                        {
                            approaches.insert(raw_exits[&exit]);
                        }
                        Source::Exit(_) => {}
                    }
                }
            }
            if approaches.len() == 1 {
                unite(
                    parent,
                    index(entry),
                    *approaches
                        .first()
                        .expect("projected topology contains the required vertex or loop endpoint"),
                );
            } else {
                minima.push((index(entry), approaches.into_iter().collect::<Vec<_>>()));
            }
        }
        for group in &regions.groups {
            for branch in regions.later_siblings(group) {
                for inside in regions.reserved(group, branch) {
                    for outside in regions.outside(group, branch) {
                        inequalities.push((index(inside), index(outside)));
                    }
                }
            }
        }
    }
    (inequalities, minima)
}

impl Columns {
    #[allow(clippy::too_many_lines)]
    fn of(flow: &Flow, topology: &Topology, flexible: bool) -> Option<Self> {
        let index = |vertex| index_of(topology, vertex);
        let n = topology.vertices.len();
        let raw_exits = topology
            .exits
            .iter()
            .enumerate()
            .map(|(i, e)| (e.id, n + i))
            .collect::<BTreeMap<_, _>>();
        let back_edge_start = n + raw_exits.len();
        let count = topology.loops.len();
        let mut parent = (0..back_edge_start + 2 * count).collect::<Vec<_>>();
        if !flexible {
            for i in 0..count {
                unite(
                    &mut parent,
                    back_edge_start + i,
                    back_edge_start + count + i,
                );
            }
        }
        serial_identities(topology, &raw_exits, &mut parent);
        let (inequalities, minima) = branch_rules(flow, topology, &raw_exits, &mut parent);
        let mut ids = BTreeMap::new();
        let groups = (0..parent.len())
            .map(|i| {
                let next = ids.len();
                *ids.entry(root(&parent, i)).or_insert(next)
            })
            .collect::<Vec<_>>();
        let mut base = Order::new(ids.len());
        for (left, right) in inequalities {
            if !base.insert(groups[left], groups[right], true) {
                return None;
            }
        }
        for i in 0..count {
            if !base.insert(
                groups[back_edge_start + i],
                groups[back_edge_start + count + i],
                false,
            ) {
                return None;
            }
        }
        let minima = minima
            .into_iter()
            .map(|(entry, approaches)| {
                (
                    groups[entry],
                    approaches
                        .into_iter()
                        .map(|a| groups[a])
                        .collect::<BTreeSet<_>>()
                        .into_iter()
                        .collect::<Vec<_>>(),
                )
            })
            .collect::<Vec<_>>();
        for (entry, approaches) in &minima {
            if approaches.is_empty() {
                return None;
            }
            for &approach in approaches {
                if !base.insert(*entry, approach, false) {
                    return None;
                }
            }
        }
        let bodies = topology
            .loops
            .iter()
            .map(|loop_| {
                let end = flow.blocks[loop_.header]
                    .loop_end
                    .expect("projected topology contains the required vertex or loop endpoint");
                std::array::from_fn(|side| {
                    super::loop_block::body_vertices(flow, topology, loop_.header)
                        .into_iter()
                        .map(|vertex| groups[index(vertex)])
                        .chain(
                            topology
                                .loops
                                .iter()
                                .enumerate()
                                .filter(|(_, inner)| {
                                    (loop_.header + 1..end).contains(&inner.header)
                                })
                                .map(|(i, _)| groups[back_edge_start + side * count + i]),
                        )
                        .collect()
                })
            })
            .collect();
        Some(Self {
            vertex: groups[..n].to_vec(),
            exits: raw_exits.into_iter().map(|(e, i)| (e, groups[i])).collect(),
            back_edges: groups[back_edge_start..back_edge_start + count].to_vec(),
            back_edge_right: groups[back_edge_start + count..].to_vec(),
            base,
            minima,
            bodies,
        })
    }

    fn settle(&self, order: &Order, at: usize) -> Option<Order> {
        let Some((entry, approaches)) = self.minima.get(at) else {
            return Some(order.clone());
        };
        for &approach in approaches {
            let mut next = order.clone();
            if next.equal(*entry, approach)
                && let Some(settled) = self.settle(&next, at + 1)
            {
                return Some(settled);
            }
        }
        None
    }
}

struct Sweep<'a> {
    /// Try completing inner back edges before paths that only finish the flow.
    /// This changes which witness is found first, never which states exist.
    visit_order: Vec<usize>,
    events: Vec<Vec<usize>>,
    flow: &'a Flow,
    topology: &'a Topology,
    columns: Columns,
    arrivals: Vec<Vec<usize>>,
    departures: Vec<Vec<Vec<usize>>>,
    predecessors: Vec<Vec<usize>>,
    entry_of: Vec<Option<usize>>,
    tail_of: Vec<Option<usize>>,
    paths: Vec<Vec<bool>>,
    barriers: Vec<Vec<bool>>,
}

#[derive(Clone)]
struct State {
    placed: Vec<bool>,
    frontier: Vec<Lifeline>,
    sides: Vec<Option<Side>>,
    order: Order,
}

type Key = (Vec<bool>, Vec<Lifeline>, Vec<Option<Side>>, Order);

pub(super) enum Refusal {
    Impossible(Obstruction),
    Internal(String),
}

pub(super) fn search(
    flow: &Flow,
    merges: &[WireMerge],
    topology: &Topology,
) -> Result<Arrangement, Refusal> {
    let Some(mut sweep) = Sweep::of(flow, topology, false) else {
        return Err(Refusal::Impossible(super::unarrangeable(flow)));
    };
    match decide(&sweep, merges) {
        Err(Refusal::Impossible(_)) => {
            sweep.columns = Columns::of(flow, topology, true)
                .expect("separating back edge bounds cannot contradict the static constraints");
            decide(&sweep, merges)
        }
        found => found,
    }
}

#[cfg(test)]
pub(super) fn search_shape(
    flow: &Flow,
    merges: &[WireMerge],
    topology: &Topology,
    flexible: bool,
) -> Result<Arrangement, Refusal> {
    let Some(sweep) = Sweep::of(flow, topology, flexible) else {
        return Err(Refusal::Impossible(super::unarrangeable(flow)));
    };
    decide(&sweep, merges)
}

fn decide(sweep: &Sweep<'_>, merges: &[WireMerge]) -> Result<Arrangement, Refusal> {
    let (flow, topology) = (sweep.flow, sweep.topology);
    let mut state = State {
        placed: vec![false; topology.vertices.len()],
        frontier: Vec::new(),
        sides: vec![None; topology.loops.len()],
        order: sweep.columns.base.clone(),
    };
    let mut deepest = (0, None);
    if let Some(arrangement) = sweep.walk(
        &mut state,
        &mut Vec::new(),
        &mut BTreeSet::new(),
        &mut deepest,
        true,
    )? {
        Ok(arrangement)
    } else {
        let vertex = deepest.1.map(|i| topology.vertices[i]);
        let mut obstruction = super::unarrangeable(flow);
        if let Some(vertex) = vertex {
            obstruction.span = super::describe::vertex_span(flow, topology, vertex);
            obstruction.message = format!(
                "its routes and column constraints have no conforming arrangement; the exhaustive search reached {} before exhausting its alternatives",
                super::describe::vertex(flow, merges, topology, vertex)
            );
        }
        Err(Refusal::Impossible(obstruction))
    }
}

#[derive(Clone, Copy)]
struct Anchor {
    left: usize,
    right: usize,
    back_edge_index: Option<usize>,
}

impl Anchor {
    fn fixed(column: usize) -> Self {
        Self {
            left: column,
            right: column,
            back_edge_index: None,
        }
    }
}

impl<'a> Sweep<'a> {
    fn of(flow: &'a Flow, topology: &'a Topology, flexible: bool) -> Option<Self> {
        let n = topology.vertices.len();
        let index = |v| {
            topology
                .vertices
                .binary_search(&v)
                .expect("projected topology contains the required vertex or loop endpoint")
        };
        let mut arrivals = vec![Vec::new(); n];
        let mut predecessors = vec![BTreeSet::new(); n];
        let mut exits = vec![BTreeMap::<Source, Vec<usize>>::new(); n];
        for (i, wire) in topology.connections.iter().enumerate() {
            arrivals[index(wire.destination)].push(i);
            predecessors[index(wire.destination)].insert(index(Vertex::from(wire.source)));
            exits[index(Vertex::from(wire.source))]
                .entry(wire.source)
                .or_default()
                .push(i);
        }
        for edge in &topology.order {
            predecessors[index(edge.destination)].insert(index(Vertex::from(edge.source)));
        }
        let mut entry_of = vec![None; n];
        let mut tail_of = vec![None; n];
        for (i, loop_) in topology.loops.iter().enumerate() {
            entry_of[index(Vertex::Junction(loop_.entry))] = Some(i);
            tail_of[index(Vertex::Junction(loop_.tail))] = Some(i);
            predecessors[index(Vertex::Junction(loop_.tail))]
                .insert(index(Vertex::Junction(loop_.entry)));
        }
        let mut paths = vec![vec![false; n]; n];
        let mut precedence = paths.clone();
        for wire in &topology.connections {
            paths[index(Vertex::from(wire.source))][index(wire.destination)] = true;
        }
        for loop_ in &topology.loops {
            paths[index(Vertex::Junction(loop_.entry))][index(Vertex::Junction(loop_.tail))] = true;
        }
        for (v, incoming) in predecessors.iter().enumerate() {
            for &p in incoming {
                precedence[p][v] = true;
            }
        }
        close_paths(&mut paths);
        close_paths(&mut precedence);
        for (v, row) in precedence.iter_mut().enumerate() {
            row[v] = false;
        }
        let barriers = (0..n)
            .map(|b| {
                (0..n)
                    .map(|v| !paths[b][v] && (0..n).any(|t| paths[b][t] && precedence[v][t]))
                    .collect()
            })
            .collect();
        let mut visit_order = (0..n).collect::<Vec<_>>();
        visit_order.sort_by_key(|&v| {
            std::cmp::Reverse(
                tail_of
                    .iter()
                    .enumerate()
                    .filter(|&(tail, _)| paths[v][tail])
                    .filter_map(|(_, &index)| index)
                    .max(),
            )
        });
        Some(Self {
            visit_order,
            events: super::choice::events(topology),
            flow,
            topology,
            columns: Columns::of(flow, topology, flexible)?,
            arrivals,
            predecessors: predecessors
                .into_iter()
                .map(|p| p.into_iter().collect())
                .collect(),
            departures: exits
                .into_iter()
                .map(|e| e.into_values().collect())
                .collect(),
            entry_of,
            tail_of,
            paths,
            barriers,
        })
    }

    fn consumed(&self, state: &State, vertex: usize) -> Option<(usize, usize)> {
        let incoming = self.events[vertex]
            .iter()
            .flat_map(|&v| &self.arrivals[v])
            .map(|&i| Lifeline::Wire(i))
            .collect::<Vec<_>>();
        let mut wanted = incoming.iter().copied().collect::<BTreeSet<_>>();
        if let Some(i) = self.tail_of[vertex] {
            wanted.insert(Lifeline::BackEdge(i));
        }
        if wanted.is_empty() {
            return state.frontier.is_empty().then_some((0, 0));
        }
        let position = state
            .frontier
            .iter()
            .position(|item| wanted.contains(item))?;
        let end = position + wanted.len();
        (end <= state.frontier.len()
            && state.frontier[position..end]
                .iter()
                .all(|item| wanted.contains(item))
            && (self.events[vertex].len() == 1 || state.frontier[position..end] == incoming))
            .then_some((position, wanted.len()))
    }

    fn emissions(&self, vertex: usize) -> Vec<Lifeline> {
        self.events[vertex]
            .iter()
            .flat_map(|&v| self.emissions_of(v))
            .collect()
    }

    fn emissions_of(&self, vertex: usize) -> Vec<Lifeline> {
        let owner = match self.topology.vertices[vertex] {
            Vertex::Node(NodeId::Block(block) | NodeId::Case { choice: block, .. }) => Some(block),
            Vertex::Node(NodeId::Start) | Vertex::Junction(_) => None,
        };
        let loop_ = self.topology.loops.iter().rev().find(|loop_| {
            owner.is_some_and(|block| {
                (loop_.header + 1
                    ..self.flow.blocks[loop_.header]
                        .loop_end
                        .expect("a loop owns its body"))
                    .contains(&block)
            })
        });
        let joins = (0..self.topology.vertices.len())
            .filter(|&i| self.arrivals[i].len() > 1 || self.tail_of[i].is_some())
            .collect::<Vec<_>>();
        self.departures[vertex]
            .iter()
            .flat_map(|group| {
                let mut group = group.clone();
                if !group.iter().all(|&wire| {
                    let connection = self.topology.connections[wire];
                    super::choice::case_destination(connection.source, connection.destination)
                        .is_some()
                }) {
                    group.sort_by_key(|&wire| {
                        let destination =
                            index_of(self.topology, self.topology.connections[wire].destination);
                        let flank = loop_.is_some_and(|loop_| {
                            let repeats = self.paths[destination]
                                [index_of(self.topology, Vertex::Junction(loop_.tail))];
                            repeats != loop_.prefer_left
                        });
                        (
                            flank,
                            joins
                                .iter()
                                .map(|&v| self.paths[destination][v])
                                .collect::<Vec<_>>(),
                        )
                    });
                }
                group.into_iter().map(Lifeline::Wire)
            })
            .collect()
    }

    fn exchangeable(&self, left: Lifeline, right: Lifeline) -> bool {
        let (Lifeline::Wire(a), Lifeline::Wire(b)) = (left, right) else {
            return false;
        };
        let (a, b) = (self.topology.connections[a], self.topology.connections[b]);
        (a.source == b.source && super::choice::case_destination(a.source, a.destination).is_none())
            || a.destination == b.destination
    }

    /// Static anchors on the event row. Untouched forward routes may move
    /// between events; back edges choose a position inside their persistent bounds.
    fn back_edge_anchor(&self, i: usize) -> Anchor {
        Anchor {
            left: self.columns.back_edges[i],
            right: self.columns.back_edge_right[i],
            back_edge_index: Some(i),
        }
    }

    fn anchors(&self, frontier: &[Lifeline], sides: &[Option<Side>], step: &Step) -> Vec<Anchor> {
        let take = |items: &[Lifeline]| {
            items
                .iter()
                .filter_map(|item| match item {
                    Lifeline::BackEdge(i) => Some(self.back_edge_anchor(*i)),
                    Lifeline::Wire(_) => None,
                })
                .collect::<Vec<_>>()
        };
        let Some(vertex) = step.vertex else {
            return take(frontier);
        };
        let mut event = Vec::new();
        for &v in &self.events[vertex] {
            event.push(Anchor::fixed(self.columns.vertex[v]));
            if let Vertex::Node(node) = self.topology.vertices[v] {
                event.extend(
                    self.topology
                        .exits
                        .iter()
                        .filter(|e| e.id.node == node && e.id.branch.is_some_and(|b| b > 0))
                        .map(|e| Anchor::fixed(self.columns.exits[&e.id])),
                );
            }
        }
        if let Some(i) = self.entry_of[vertex].or(self.tail_of[vertex]) {
            match step
                .side
                .or(sides[i])
                .expect("a loop side is chosen at entry")
            {
                Side::Left => event.insert(0, self.back_edge_anchor(i)),
                Side::Right => event.push(self.back_edge_anchor(i)),
            }
        }
        let mut anchors = take(&frontier[..step.position]);
        anchors.extend(event);
        anchors.extend(take(&frontier[step.position + step.consumed.len()..]));
        anchors
    }

    fn constrain(&self, state: &State, step: &Step) -> Option<Order> {
        let vertex = step.vertex.expect("a vertex event");
        let mut order = state.order.clone();
        if let Some(i) = self.entry_of[vertex] {
            let side = step
                .side
                .expect("projected topology contains the required vertex or loop endpoint");
            let bound = match side {
                Side::Left => self.columns.back_edge_right[i],
                Side::Right => self.columns.back_edges[i],
            };
            let flank = usize::from(side == Side::Right);
            for &body in &self.columns.bodies[i][flank] {
                let (left, right) = match side {
                    Side::Left => (bound, body),
                    Side::Right => (body, bound),
                };
                if !order.insert(left, right, true) {
                    return None;
                }
            }
        }
        if let Some(i) = self.tail_of[vertex] {
            let outside = match state.sides[i]
                .expect("projected topology contains the required vertex or loop endpoint")
            {
                Side::Left => step.consumed.first(),
                Side::Right => step.consumed.last(),
            };
            if outside != Some(&Lifeline::BackEdge(i)) {
                return None;
            }
        }
        let anchors = self.anchors(&state.frontier, &state.sides, step);
        for (i, left) in anchors.iter().enumerate() {
            for right in &anchors[i + 1..] {
                if !order.insert(left.left, right.right, true) {
                    return None;
                }
            }
        }
        Some(order)
    }

    /// A path to a vertex strictly after v separates paths to v on its two
    /// sides, provided it cannot itself reach v. It cannot meet either path
    /// before v: such a meeting would make it an ancestor of v. Only the
    /// already live edges may escape through their common source; exclude
    /// those from this obstruction. See RFC 0003 §2.1.
    fn sealed(&self, state: &State) -> bool {
        let destination = |item: Lifeline| match item {
            Lifeline::Wire(w) => self.topology.connections[w].destination,
            Lifeline::BackEdge(i) => Vertex::Junction(self.topology.loops[i].tail),
        };
        let ends = state
            .frontier
            .iter()
            .map(|&a| {
                self.topology
                    .vertices
                    .binary_search(&destination(a))
                    .expect("a projected endpoint")
            })
            .collect::<Vec<_>>();
        for (v, placed) in state.placed.iter().enumerate() {
            if *placed {
                continue;
            }
            for (at, &barrier) in state.frontier.iter().enumerate() {
                if !self.barriers[ends[at]][v] {
                    continue;
                }
                let reaches = |i: usize| {
                    self.paths[ends[i]][v] && !self.exchangeable(state.frontier[i], barrier)
                };
                if (0..at).any(reaches) && (at + 1..ends.len()).any(reaches) {
                    return true;
                }
            }
        }
        false
    }

    fn walk(
        &self,
        state: &mut State,
        steps: &mut Vec<Step>,
        failed: &mut BTreeSet<Key>,
        deepest: &mut (usize, Option<usize>),
        memo: bool,
    ) -> Result<Option<Arrangement>, Refusal> {
        if state.placed.iter().all(|p| *p) {
            let Some(order) = self.columns.settle(&state.order, 0) else {
                return Ok(None);
            };
            let built = self.expand(steps, &order);
            super::verify::arrangement(self.flow, self.topology, &built)
                .map_err(Refusal::Internal)?;
            return Ok(Some(built));
        }
        if memo && self.sealed(state) {
            return Ok(None);
        }
        let key = (
            state.placed.clone(),
            state.frontier.clone(),
            state.sides.clone(),
            state.order.clone(),
        );
        if memo && failed.contains(&key) {
            return Ok(None);
        }
        // A cut may change order only where two forward routes are allowed
        // to share a segment. Enumerate its finite exchange orbit, keeping one
        // shortest trace for each cut. Exchanges add no persistent x constraint.
        let original = state.frontier.clone();
        let mut queue = vec![(original.clone(), None::<(usize, usize)>)];
        let mut seen = BTreeSet::from([original.clone()]);
        let mut at = 0;
        while at < queue.len() {
            let cut = queue[at].0.clone();
            let mut chain = Vec::new();
            let mut cursor = at;
            while let Some((parent, position)) = queue[cursor].1 {
                chain.push(Step {
                    vertex: None,
                    position,
                    consumed: queue[parent].0[position..position + 2].to_vec(),
                    emitted: queue[cursor].0[position..position + 2].to_vec(),
                    side: None,
                });
                cursor = parent;
            }
            chain.reverse();
            state.frontier.clone_from(&cut);
            let kept = steps.len();
            steps.extend(chain);
            let result = self.place(state, steps, failed, deepest, memo);
            steps.truncate(kept);
            state.frontier.clone_from(&original);
            if !matches!(&result, Ok(None)) {
                return result;
            }
            for position in 0..cut.len().saturating_sub(1) {
                if !self.exchangeable(cut[position], cut[position + 1]) {
                    continue;
                }
                let mut next = cut.clone();
                next.swap(position, position + 1);
                if seen.insert(next.clone()) {
                    queue.push((next, Some((at, position))));
                }
            }
            at += 1;
        }
        // Memoization is optional: stop growing the cache after 64 MiB of
        // estimated payload. Omitting an entry repeats work, never rejects a
        // continuation or changes the finite exhaustive search.
        let bytes = state.order.relation.len()
            + state.frontier.len() * size_of::<Lifeline>()
            + state.placed.len()
            + state.sides.len() * size_of::<Option<Side>>()
            + size_of::<Key>()
            + 128;
        if memo && failed.len() < 64 * 1024 * 1024 / bytes.max(1) {
            failed.insert(key);
        }
        Ok(None)
    }

    fn place(
        &self,
        state: &mut State,
        steps: &mut Vec<Step>,
        failed: &mut BTreeSet<Key>,
        deepest: &mut (usize, Option<usize>),
        memo: bool,
    ) -> Result<Option<Arrangement>, Refusal> {
        for &vertex in &self.visit_order {
            if self.events[vertex].is_empty()
                || state.placed[vertex]
                || self.events[vertex]
                    .iter()
                    .any(|&v| self.predecessors[v].iter().any(|&i| !state.placed[i]))
            {
                continue;
            }
            if steps.len() >= deepest.0 {
                *deepest = (steps.len(), Some(vertex));
            }
            let Some((position, count)) = self.consumed(state, vertex) else {
                continue;
            };
            let consumed = state.frontier[position..position + count].to_vec();
            let sides = if let Some(i) = self.entry_of[vertex] {
                if self.topology.loops[i].prefer_left {
                    vec![Some(Side::Left), Some(Side::Right)]
                } else {
                    vec![Some(Side::Right), Some(Side::Left)]
                }
            } else {
                vec![None]
            };
            for side in sides {
                {
                    let mut emitted = self.emissions(vertex);
                    if let Some(i) = self.entry_of[vertex] {
                        match side.expect(
                            "projected topology contains the required vertex or loop endpoint",
                        ) {
                            Side::Left => emitted.insert(0, Lifeline::BackEdge(i)),
                            Side::Right => emitted.push(Lifeline::BackEdge(i)),
                        }
                    }
                    let step = Step {
                        vertex: Some(vertex),
                        position,
                        consumed: consumed.clone(),
                        emitted,
                        side,
                    };
                    let Some(order) = self.constrain(state, &step) else {
                        continue;
                    };
                    let kept = state.clone();
                    state.order = order;
                    for &v in &self.events[vertex] {
                        state.placed[v] = true;
                    }
                    if let Some(i) = self.entry_of[vertex] {
                        state.sides[i] = side;
                    }
                    state
                        .frontier
                        .splice(position..position + count, step.emitted.iter().copied());
                    steps.push(step);
                    let found = self.walk(state, steps, failed, deepest, memo);
                    steps.pop();
                    *state = kept;
                    if !matches!(&found, Ok(None)) {
                        return found;
                    }
                }
            }
        }
        Ok(None)
    }

    fn expand(&self, steps: &[Step], order: &Order) -> Arrangement {
        let room = self.topology.connections.len() as i32 + 2;
        let values = order
            .number()
            .into_iter()
            .map(|x| {
                x * (4
                    * room
                    * (self.topology.connections.len() + self.topology.loops.len() + 2) as i32)
            })
            .collect::<Vec<_>>();
        let mut built = self.empty_arrangement(steps, &values);
        let mut frontier = Vec::new();
        let mut previous = BTreeMap::new();
        let mut sides = vec![None; self.topology.loops.len()];
        for (rank, step) in steps.iter().enumerate() {
            if let Some(i) = step.vertex.and_then(|v| self.entry_of[v]) {
                sides[i] = step.side;
                built.contours[i].side = step.side.expect("an entry chooses its side");
            }
            let mut spines = vec![0; self.topology.loops.len()];
            let anchors = self.anchors(&frontier, &sides, step);
            let mut at = anchors.first().map_or(0, |a| values[a.left] - 4 * room);
            for anchor in anchors {
                at = (at + 4 * room).max(values[anchor.left]);
                assert!(
                    at <= values[anchor.right],
                    "ordered intervals have enough room"
                );
                if let Some(i) = anchor.back_edge_index {
                    spines[i] = at;
                }
            }
            let (before, after) = self.row(step, &frontier, &values, &spines, room, &previous);
            if let Some(i) = step.vertex.and_then(|v| self.entry_of[v]) {
                let x = after[&Lifeline::BackEdge(i)];
                built.contours[i].column = x;
                built.back_routes.insert(
                    i,
                    Route {
                        departure: x,
                        arrival: x,
                        runs: Vec::new(),
                    },
                );
            }
            if let Some(i) = step.vertex.and_then(|v| self.tail_of[v]) {
                built
                    .back_routes
                    .get_mut(&i)
                    .expect("the back edge entered before its tail")
                    .arrival = before[&Lifeline::BackEdge(i)];
            }
            if rank > 0 {
                Self::strip(
                    rank - 1,
                    &steps[rank - 1],
                    step,
                    &frontier,
                    &previous,
                    &before,
                    &mut built,
                );
            }
            frontier.splice(
                step.position..step.position + step.consumed.len(),
                step.emitted.iter().copied(),
            );
            previous = after;
        }
        built.back_routes.retain(|_, route| !route.runs.is_empty());
        compress(&mut built);
        built
    }

    fn empty_arrangement(&self, steps: &[Step], values: &[i32]) -> Arrangement {
        Arrangement {
            rank: steps
                .iter()
                .enumerate()
                .flat_map(|(r, s)| {
                    s.vertex.into_iter().flat_map(move |v| {
                        self.events[v]
                            .iter()
                            .map(move |&v| (self.topology.vertices[v], r))
                    })
                })
                .collect(),
            ranks: steps.len(),
            column: self
                .topology
                .vertices
                .iter()
                .enumerate()
                .map(|(i, &v)| (v, values[self.columns.vertex[i]]))
                .collect(),
            exit_offset: self
                .columns
                .exits
                .iter()
                .map(|(&e, &i)| {
                    let own = self
                        .topology
                        .vertices
                        .binary_search(&Vertex::Node(e.node))
                        .expect("projected topology contains the required vertex or loop endpoint");
                    (e, values[i] - values[self.columns.vertex[own]])
                })
                .collect(),
            routes: self
                .topology
                .connections
                .iter()
                .map(|wire| {
                    let arrival = values[self.columns.vertex[self
                        .topology
                        .vertices
                        .binary_search(&wire.destination)
                        .expect(
                            "projected topology contains the required vertex or loop endpoint",
                        )]];
                    let departure = if super::choice::case_destination(
                        wire.source,
                        wire.destination,
                    )
                    .is_some()
                    {
                        arrival
                    } else {
                        match wire.source {
                            Source::Exit(exit) => values[self.columns.exits[&exit]],
                            Source::Junction(j) => values[self.columns.vertex[self
                                .topology
                                .vertices
                                .binary_search(&Vertex::Junction(j))
                                .expect(
                                    "projected topology contains the required vertex or loop endpoint",
                                )]],
                        }
                    };
                    Route {
                        departure,
                        arrival,
                        runs: Vec::new(),
                    }
                })
                .collect(),
            gap_lanes: vec![0; steps.len()],
            back_routes: BTreeMap::new(),
            contours: vec![
                Contour {
                    side: Side::Left,
                    column: 0,
                    lane: 0
                };
                self.topology.loops.len()
            ],
        }
    }

    /// Coordinates at a vertex event, with one spare integer per forward
    /// route on each side of every static anchor. The event's incident routes
    /// fan out or merge inside that spare space.
    fn row(
        &self,
        step: &Step,
        frontier: &[Lifeline],
        values: &[i32],
        spines: &[i32],
        room: i32,
        previous: &BTreeMap<Lifeline, i32>,
    ) -> (BTreeMap<Lifeline, i32>, BTreeMap<Lifeline, i32>) {
        let Some(vertex) = step.vertex else {
            return Self::exchange_row(step, frontier, spines, room);
        };
        let own = values[self.columns.vertex[vertex]];
        let port = |wire: usize| {
            let connection = self.topology.connections[wire];
            if let Some(case) =
                super::choice::case_destination(connection.source, connection.destination)
            {
                values[self.columns.vertex[index_of(self.topology, Vertex::Node(case))]]
            } else {
                match connection.source {
                    Source::Exit(exit) => values[self.columns.exits[&exit]],
                    Source::Junction(_) => own,
                }
            }
        };
        let mut event_anchors = self.events[vertex]
            .iter()
            .map(|&v| values[self.columns.vertex[v]])
            .collect::<Vec<_>>();
        event_anchors.extend(step.emitted.iter().map(|item| match item {
            Lifeline::Wire(w) => port(*w),
            Lifeline::BackEdge(i) => spines[*i],
        }));
        if let Some(i) = self.tail_of[vertex] {
            event_anchors.push(spines[i]);
        }
        let (left, right) = event_anchors
            .into_iter()
            .fold((own, own), |(left, right), x| (left.min(x), right.max(x)));
        let (left, right) = (left - room, right + room);
        let mut fixed = BTreeMap::new();
        for (slice, bound, before_event) in [
            (&frontier[..step.position], left, true),
            (
                &frontier[step.position + step.consumed.len()..],
                right,
                false,
            ),
        ] {
            pack_side(
                slice,
                bound,
                before_event,
                spines,
                room,
                previous,
                &mut fixed,
            );
        }
        let mut before = fixed.clone();
        let incoming = self.arrivals[vertex].len();
        let start =
            if super::serial_arrival(self.topology, self.topology.vertices[vertex]).is_some() {
                own
            } else if self.tail_of[vertex]
                .is_some_and(|i| step.consumed.last() == Some(&Lifeline::BackEdge(i)))
            {
                own - incoming as i32
            } else {
                own + 1
            };
        let mut offset = 0;
        for &item in &step.consumed {
            let x = match item {
                Lifeline::BackEdge(i) => spines[i],
                Lifeline::Wire(w) if self.events[vertex].len() > 1 => {
                    values[self.columns.vertex
                        [index_of(self.topology, self.topology.connections[w].destination)]]
                }
                Lifeline::Wire(_) => {
                    let x = start + offset;
                    offset += 1;
                    x
                }
            };
            before.insert(item, x);
        }
        let mut after = fixed;
        let mut counts = BTreeMap::new();
        for &item in &step.emitted {
            let x = match item {
                Lifeline::BackEdge(i) => spines[i],
                Lifeline::Wire(w) => {
                    let at = port(w);
                    let count = counts.entry(at).or_insert(0);
                    let x = at + *count;
                    *count += 1;
                    x
                }
            };
            after.insert(item, x);
        }
        (before, after)
    }

    fn exchange_row(
        step: &Step,
        frontier: &[Lifeline],
        spines: &[i32],
        room: i32,
    ) -> (BTreeMap<Lifeline, i32>, BTreeMap<Lifeline, i32>) {
        let mut before = BTreeMap::new();
        let mut at = frontier
            .iter()
            .find_map(|item| match item {
                Lifeline::BackEdge(i) => Some(spines[*i] - room),
                Lifeline::Wire(_) => None,
            })
            .unwrap_or(0)
            - room;
        for &item in frontier {
            if let Lifeline::BackEdge(i) = item {
                at = spines[i];
            }
            before.insert(item, at);
            at += 1;
        }
        let mut after = before.clone();
        after.insert(step.consumed[0], before[&step.consumed[1]]);
        after.insert(step.consumed[1], before[&step.consumed[0]]);
        (before, after)
    }

    /// Ordered wires can be moved without crossing: leftward moves from left
    /// to right, then rightward moves from right to left. Shared departures
    /// split before these moves; arrivals merge on one final common rail.
    fn strip(
        gap: usize,
        above: &Step,
        below: &Step,
        frontier: &[Lifeline],
        start: &BTreeMap<Lifeline, i32>,
        target: &BTreeMap<Lifeline, i32>,
        built: &mut Arrangement,
    ) {
        let mut lane = 0;
        for &item in above.emitted.iter().filter(|_| above.vertex.is_some()) {
            if let Lifeline::Wire(w) = item {
                let departure = built.routes[w].departure;
                add_run(&mut built.routes[w], gap, lane, departure, start[&item]);
            }
        }
        lane += 1;
        for reverse in [false, true] {
            let mut order = frontier.to_vec();
            if reverse {
                order.reverse();
            }
            for item in order {
                let (from, to) = (start[&item], target[&item]);
                if (to > from && reverse) || (to < from && !reverse) {
                    let route = match item {
                        Lifeline::Wire(w) => &mut built.routes[w],
                        Lifeline::BackEdge(i) => built
                            .back_routes
                            .get_mut(&i)
                            .expect("a live iteration back edge"),
                    };
                    add_run(route, gap, lane, from, to);
                    lane += 1;
                }
            }
        }
        for &item in &below.consumed {
            if let Lifeline::Wire(w) = item {
                let arrival = if below.vertex.is_some() {
                    built.routes[w].arrival
                } else {
                    let other = below
                        .consumed
                        .iter()
                        .find(|&&item| item != Lifeline::Wire(w))
                        .expect("two exchanging wires");
                    target[other]
                };
                add_run(&mut built.routes[w], gap, lane, target[&item], arrival);
            }
        }
        built.gap_lanes[gap] = lane + 1;
    }
}

/// Pack paths between back edge spines on one side of a vertex event.
fn pack_side(
    slice: &[Lifeline],
    bound: i32,
    before_event: bool,
    spines: &[i32],
    room: i32,
    previous: &BTreeMap<Lifeline, i32>,
    fixed: &mut BTreeMap<Lifeline, i32>,
) {
    let mut at = if before_event {
        slice
            .iter()
            .filter_map(|wire| previous.get(wire))
            .copied()
            .min()
            .unwrap_or(bound)
            .min(bound)
            - room
    } else {
        bound
    };
    let mut run = Vec::new();
    for &item in slice {
        if let Lifeline::BackEdge(i) = item {
            let target = spines[i];
            let start = at.min(target - room - run.len() as i32);
            pack(&run, start, target - room, previous, fixed);
            run.clear();
            fixed.insert(item, target);
            at = target + room;
        } else {
            run.push(item);
        }
    }
    let end = if before_event {
        bound
    } else {
        run.iter()
            .filter_map(|wire| previous.get(wire))
            .copied()
            .max()
            .unwrap_or(at)
            .max(at)
            + run.len() as i32
            + room
    };
    let start = at.min(end - run.len() as i32);
    pack(&run, start, end, previous, fixed);
}

/// Keep a live path where it already stood whenever the new anchor interval
/// leaves room. Clamping left to right preserves order and reserves one integer
/// for each remaining path; it changes coordinates, never the search state.
fn pack(
    paths: &[Lifeline],
    mut left: i32,
    right: i32,
    previous: &BTreeMap<Lifeline, i32>,
    placed: &mut BTreeMap<Lifeline, i32>,
) {
    for (i, &wire) in paths.iter().enumerate() {
        let last = right - (paths.len() - i) as i32;
        let x = previous
            .get(&wire)
            .copied()
            .unwrap_or(left)
            .clamp(left, last);
        placed.insert(wire, x);
        left = x + 1;
    }
}

/// Reachability has no distances: one machine word propagates 64 vertices.
fn close_paths(paths: &mut [Vec<bool>]) {
    let words = paths.len().div_ceil(64);
    let mut packed = paths
        .iter()
        .map(|row| {
            let mut bits = vec![0_u64; words];
            for (v, &reachable) in row.iter().enumerate() {
                if reachable {
                    bits[v / 64] |= 1 << (v % 64);
                }
            }
            bits
        })
        .collect::<Vec<_>>();
    for k in 0..paths.len() {
        let through = packed[k].clone();
        for row in &mut packed {
            if row[k / 64] & (1 << (k % 64)) != 0 {
                for (word, next) in row.iter_mut().zip(&through) {
                    *word |= next;
                }
            }
        }
    }
    for (i, (row, bits)) in paths.iter_mut().zip(packed).enumerate() {
        for (v, reachable) in row.iter_mut().enumerate() {
            *reachable = i == v || bits[v / 64] & (1 << (v % 64)) != 0;
        }
    }
}

fn add_run(route: &mut Route, gap: usize, lane: usize, enter: i32, exit: i32) {
    if enter != exit {
        route.runs.push(Run {
            gap,
            enter,
            exit,
            lane,
        });
    }
}

pub(super) fn compress(built: &mut Arrangement) {
    let coordinates = built
        .column
        .values()
        .copied()
        .chain(
            built
                .exit_offset
                .iter()
                .map(|(exit, offset)| built.column[&Vertex::Node(exit.node)] + offset),
        )
        .chain(
            built
                .routes
                .iter()
                .chain(built.back_routes.values())
                .flat_map(|r| [r.departure, r.arrival]),
        )
        .chain(
            built
                .routes
                .iter()
                .chain(built.back_routes.values())
                .flat_map(|r| r.runs.iter().flat_map(|s| [s.enter, s.exit])),
        )
        .chain(built.contours.iter().map(|c| c.column))
        .collect::<BTreeSet<_>>();
    let numbered = coordinates
        .into_iter()
        .enumerate()
        .map(|(i, x)| (x, i as i32))
        .collect::<BTreeMap<_, _>>();
    for (exit, offset) in &mut built.exit_offset {
        let column = built.column[&Vertex::Node(exit.node)];
        *offset = numbered[&(column + *offset)] - numbered[&column];
    }
    for column in built.column.values_mut() {
        *column = numbered[column];
    }
    for contour in &mut built.contours {
        contour.column = numbered[&contour.column];
    }
    let mut used = built
        .routes
        .iter()
        .chain(built.back_routes.values())
        .flat_map(|route| route.runs.iter().map(|run| (run.gap, run.lane)))
        .collect::<BTreeSet<_>>();
    let bent_gaps = used.iter().map(|&(gap, _)| gap).collect::<BTreeSet<_>>();
    for (&vertex, &rank) in &built.rank {
        // A junction with a straight arrival sits on its rank's line. An
        // unused final lane can keep somebody else's bend above that line;
        // deleting it would make the bend touch the junction or its back edge.
        if matches!(vertex, Vertex::Junction(_))
            && rank > 0
            && bent_gaps.contains(&(rank - 1))
            && built.gap_lanes[rank - 1] > 0
        {
            used.insert((rank - 1, built.gap_lanes[rank - 1] - 1));
        }
    }
    let mut lanes = BTreeMap::new();
    built.gap_lanes.fill(0);
    for (gap, lane) in used {
        lanes.insert((gap, lane), built.gap_lanes[gap]);
        built.gap_lanes[gap] += 1;
    }
    for route in built
        .routes
        .iter_mut()
        .chain(built.back_routes.values_mut())
    {
        route.departure = numbered[&route.departure];
        route.arrival = numbered[&route.arrival];
        for run in &mut route.runs {
            run.lane = lanes[&(run.gap, run.lane)];
            run.enter = numbered[&run.enter];
            run.exit = numbered[&run.exit];
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalization_keeps_a_bend_above_a_junction_with_a_straight_arrival() {
        let parts =
            super::super::tests::parts_of(&super::super::tests::looping(&["repeat", "break"]))
                .unwrap();
        let junction = Vertex::Junction(parts.topology.loops[0].entry);
        let mut built = Arrangement {
            rank: BTreeMap::from([(junction, 1)]),
            ranks: 2,
            routes: vec![Route {
                departure: 0,
                arrival: 1,
                runs: vec![Run {
                    gap: 0,
                    lane: 0,
                    enter: 0,
                    exit: 1,
                }],
            }],
            gap_lanes: vec![2, 0],
            ..Arrangement::default()
        };
        let separated = |built: &Arrangement| {
            let grid = super::super::verify::Grid::of(&parts.topology, built);
            grid.lane(0, built.routes[0].runs[0].lane) < grid.rank(built.rank[&junction])
        };
        assert!(separated(&built));
        compress(&mut built);
        assert!(
            separated(&built),
            "normalization moved a bend onto the junction line"
        );
    }

    #[test]
    fn strict_cycles_are_refused_and_weak_cycles_are_equalities() {
        let mut order = Order::new(3);
        assert!(order.insert(0, 1, false));
        assert!(order.insert(1, 0, false));
        assert!(order.insert(1, 2, true));
        assert!(!order.insert(2, 0, false));
        assert_eq!(order.number(), [0, 0, 1]);
    }

    #[test]
    fn the_reductions_preserve_all_two_and_three_route_answers() {
        compare_reductions(super::super::tests::small_decision_cases());
    }

    #[test]
    #[ignore = "exhaustive; run explicitly with --release --ignored"]
    fn the_memoized_walk_agrees_with_the_whole_state_space() {
        compare_reductions(super::super::tests::decision_cases());
    }

    #[test]
    fn a_shared_rail_cannot_lower_a_case_past_its_siblings() {
        let source = super::super::tests::looping(&["repeat", "break", "repeat"]);
        let parts = super::super::tests::parts_of(&source).unwrap();
        let sweep = Sweep::of(&parts.flow, &parts.topology, true).unwrap();
        let mut state = State {
            placed: vec![false; parts.topology.vertices.len()],
            frontier: Vec::new(),
            sides: vec![None; parts.topology.loops.len()],
            order: sweep.columns.base.clone(),
        };
        let mut steps = Vec::new();
        loop {
            let vertex = (0..state.placed.len())
                .find(|&v| {
                    !state.placed[v]
                        && sweep.predecessors[v].iter().all(|&p| state.placed[p])
                        && sweep.consumed(&state, v).is_some()
                })
                .unwrap();
            let (position, count) = sweep.consumed(&state, vertex).unwrap();
            let mut emitted = sweep.departures[vertex]
                .iter()
                .flatten()
                .copied()
                .map(Lifeline::Wire)
                .collect::<Vec<_>>();
            let side = sweep.entry_of[vertex].map(|i| {
                emitted.insert(0, Lifeline::BackEdge(i));
                Side::Left
            });
            let step = Step {
                vertex: Some(vertex),
                position,
                consumed: state.frontier[position..position + count].to_vec(),
                emitted,
                side,
            };
            state.order = sweep.constrain(&state, &step).unwrap();
            state.placed[vertex] = true;
            if let Some(i) = sweep.entry_of[vertex] {
                state.sides[i] = side;
            }
            state
                .frontier
                .splice(position..position + count, step.emitted.iter().copied());
            steps.push(step);
            if sweep.departures[vertex].iter().any(|group| group.len() > 1) {
                break;
            }
        }
        let result = sweep.walk(
            &mut state,
            &mut steps,
            &mut BTreeSet::new(),
            &mut (0, None),
            true,
        );
        assert!(
            matches!(result, Ok(None)),
            "the case row must stay together"
        );
    }

    fn compare_reductions(cases: Vec<String>) {
        for source in cases {
            let Some(parts) = super::super::tests::parts_of(&source) else {
                continue;
            };
            // Reductions also run in the deciding family with variable back edge
            // positions; the positive straight back edge preference cannot hide it.
            for flexible in [false, true] {
                let Some(sweep) = Sweep::of(&parts.flow, &parts.topology, flexible) else {
                    continue;
                };
                let initial = State {
                    placed: vec![false; parts.topology.vertices.len()],
                    frontier: Vec::new(),
                    sides: vec![None; parts.topology.loops.len()],
                    order: sweep.columns.base.clone(),
                };
                let outcomes = [false, true].map(|memo| {
                    let result = sweep.walk(
                        &mut initial.clone(),
                        &mut Vec::new(),
                        &mut BTreeSet::new(),
                        &mut (0, None),
                        memo,
                    );
                    match result {
                        Ok(found) => found.is_some(),
                        Err(Refusal::Internal(reason)) => panic!("{source}\n{reason}"),
                        Err(Refusal::Impossible(_)) => unreachable!(),
                    }
                });
                assert_eq!(outcomes[0], outcomes[1], "flexible={flexible}\n{source}");
            }
        }
    }
}
