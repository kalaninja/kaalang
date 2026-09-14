//! Independent enumeration of vertex events and ordered cuts. Unlike the
//! production search, this keeps a sparse difference-constraint graph with a
//! new coordinate for every forward route and back edge on every event row.
//! Strong components and a DAG longest-path pass solve it; no production
//! index, transition, state key, numbering, or route
//! expansion is used. The shared verifier checks the resulting witness.

use super::verify;
use crate::construct::{Arrangement, Contour, Route, Run, Side};
use crate::model::{BlockKind, Flow};
use crate::topology::{Connection, ExitId, NodeId, Source, Topology, Vertex};
use std::collections::{BTreeMap, BTreeSet};

const STATES: usize = 40_000_000;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Active {
    Edge(usize),
    Back(usize),
}

#[derive(Clone)]
struct Inequalities {
    variables: usize,
    edges: Vec<(usize, usize, i32)>,
}

impl Inequalities {
    fn fresh(&mut self) -> usize {
        let i = self.variables;
        self.variables += 1;
        i
    }
    fn before(&mut self, a: usize, b: usize) {
        self.edges.push((a, b, 1));
    }
    fn equal(&mut self, a: usize, b: usize) {
        self.edges.extend([(a, b, 0), (b, a, 0)]);
    }
    fn solve(&self) -> Option<Vec<i32>> {
        let mut forward = vec![Vec::new(); self.variables];
        let mut backward = forward.clone();
        for &(a, b, _) in &self.edges {
            forward[a].push(b);
            backward[b].push(a);
        }
        let mut seen = vec![false; self.variables];
        let mut finished = Vec::new();
        for v in 0..self.variables {
            finish(v, &forward, &mut seen, &mut finished);
        }
        let mut component = vec![usize::MAX; self.variables];
        let mut count = 0;
        for &v in finished.iter().rev() {
            if component[v] != usize::MAX {
                continue;
            }
            let mut pending = vec![v];
            component[v] = count;
            while let Some(a) = pending.pop() {
                for &b in &backward[a] {
                    if component[b] == usize::MAX {
                        component[b] = count;
                        pending.push(b);
                    }
                }
            }
            count += 1;
        }
        let mut dag = vec![Vec::new(); count];
        let mut incoming = vec![0; count];
        for &(a, b, weight) in &self.edges {
            let (a, b) = (component[a], component[b]);
            if a == b {
                if weight > 0 {
                    return None;
                }
            } else {
                dag[a].push((b, weight));
                incoming[b] += 1;
            }
        }
        let mut ready = (0..count).filter(|&i| incoming[i] == 0).collect::<Vec<_>>();
        let mut value = vec![0; count];
        while let Some(a) = ready.pop() {
            for &(b, weight) in &dag[a] {
                value[b] = value[b].max(value[a] + weight);
                incoming[b] -= 1;
                if incoming[b] == 0 {
                    ready.push(b);
                }
            }
        }
        Some(component.into_iter().map(|c| value[c]).collect())
    }
}

fn finish(v: usize, graph: &[Vec<usize>], seen: &mut [bool], result: &mut Vec<usize>) {
    if seen[v] {
        return;
    }
    seen[v] = true;
    for &next in &graph[v] {
        finish(next, graph, seen, result);
    }
    result.push(v);
}

#[derive(Clone)]
struct Event {
    vertex: Option<Vertex>,
    before: BTreeMap<Active, usize>,
    after: BTreeMap<Active, usize>,
    input: Vec<Active>,
    output: Vec<Active>,
}

struct Reference<'a> {
    flow: &'a Flow,
    topology: &'a Topology,
    vertices: BTreeMap<Vertex, usize>,
    ports: BTreeMap<ExitId, usize>,
    spines: Vec<usize>,
    upper: Vec<usize>,
    initial: Inequalities,
    minima: Vec<(usize, Vec<usize>)>,
    visited: usize,
    priority: Vec<Vertex>,
    order_keys: BTreeMap<Vertex, Vec<bool>>,
    exchanges: bool,
    quick: bool,
    variation: usize,
    divisors: BTreeMap<Vertex, usize>,
    variations: usize,
}

pub(super) fn admissible(flow: &Flow, topology: &Topology) -> bool {
    witness(flow, topology).is_some()
}

pub(super) fn witness(flow: &Flow, topology: &Topology) -> Option<Arrangement> {
    let mut reference = Reference::new(flow, topology);
    let constraints = reference.initial.clone();
    // Cheap positive attempts cannot return a negative answer. Try the cycle
    // sides fairly before spending the test's time on one bad assignment.
    for variation in 0..reference.variations.min(256) {
        reference.quick = true;
        reference.visited = 0;
        reference.variation = variation;
        let mut straight = constraints.clone();
        for (&low, &high) in reference.spines.iter().zip(&reference.upper) {
            straight.equal(low, high);
        }
        if let Some(drawing) = reference.enumerate(
            &mut Vec::new(),
            &[],
            &vec![None; topology.loops.len()],
            &straight,
        ) {
            return Some(drawing);
        }
    }
    reference.quick = false;
    reference.exchanges = true;
    reference.visited = 0;
    reference.enumerate(
        &mut Vec::new(),
        &[],
        &vec![None; topology.loops.len()],
        &constraints,
    )
}

impl<'a> Reference<'a> {
    fn new(flow: &'a Flow, topology: &'a Topology) -> Self {
        let mut initial = Inequalities {
            variables: 0,
            edges: Vec::new(),
        };
        let vertices = topology
            .vertices
            .iter()
            .map(|&v| (v, initial.fresh()))
            .collect::<BTreeMap<_, _>>();
        let ports = topology
            .exits
            .iter()
            .map(|e| (e.id, initial.fresh()))
            .collect::<BTreeMap<_, _>>();
        let spines = topology
            .loops
            .iter()
            .map(|_| initial.fresh())
            .collect::<Vec<_>>();
        let upper = spines
            .iter()
            .map(|&low| {
                let high = initial.fresh();
                initial.edges.push((low, high, 0));
                high
            })
            .collect();
        for (&exit, &port) in &ports {
            if exit.branch.is_none_or(|b| b == 0) {
                initial.equal(port, vertices[&Vertex::Node(exit.node)]);
            }
        }
        for &destination in &topology.vertices {
            if matches!(destination, Vertex::Node(NodeId::Case { .. }))
                || topology
                    .loops
                    .iter()
                    .any(|loop_| destination == Vertex::Junction(loop_.tail))
            {
                continue;
            }
            let incoming = topology
                .connections
                .iter()
                .filter(|wire| wire.destination == destination)
                .collect::<Vec<_>>();
            if let [wire] = incoming.as_slice() {
                let source = match wire.source {
                    Source::Exit(exit) => ports[&exit],
                    Source::Junction(j) => vertices[&Vertex::Junction(j)],
                };
                initial.equal(source, vertices[&destination]);
            }
        }
        let (priority, order_keys) = vertex_priority(topology);
        let mut variations = 2usize.saturating_pow(topology.loops.len() as u32);
        let divisors = topology
            .vertices
            .iter()
            .map(|&v| {
                let own = variations;
                let mut counts = BTreeMap::<Source, usize>::new();
                for wire in topology
                    .connections
                    .iter()
                    .filter(|w| Vertex::from(w.source) == v)
                {
                    *counts.entry(wire.source).or_default() += 1;
                }
                for count in counts.values() {
                    for n in 2..=*count {
                        variations = variations.saturating_mul(n);
                    }
                }
                (v, own)
            })
            .collect();
        let mut built = Self {
            flow,
            topology,
            vertices,
            ports,
            spines,
            upper,
            initial,
            minima: Vec::new(),
            visited: 0,
            priority,
            order_keys,
            exchanges: false,
            quick: false,
            variation: 0,
            divisors,
            variations,
        };
        built.branch_rules();
        built
    }

    fn branch_rules(&mut self) {
        let Self {
            flow,
            topology,
            vertices,
            ports,
            initial,
            minima,
            ..
        } = self;
        let reachable = crate::construct::regions::reachable(topology);
        for block in crate::construct::regions::branchers(flow, topology) {
            let regions = crate::construct::regions::regions(flow, topology, &reachable, block);
            let branch_columns = (0..flow.blocks[block].branch_count())
                .map(|branch| match flow.blocks[block].kind {
                    BlockKind::Choice => {
                        vertices[&Vertex::Node(NodeId::Case {
                            choice: block,
                            branch,
                        })]
                    }
                    _ => {
                        ports[&ExitId {
                            node: NodeId::Block(block),
                            branch: Some(branch),
                        }]
                    }
                })
                .collect::<Vec<_>>();
            initial.equal(
                vertices[&Vertex::Node(NodeId::Block(block))],
                branch_columns[0],
            );
            for pair in branch_columns.windows(2) {
                initial.before(pair[0], pair[1]);
            }
            for (entry, first) in
                crate::construct::regions::continuations(topology, block, &regions.branches)
            {
                let mut queue = vec![entry];
                let mut seen = BTreeSet::new();
                let mut approaches = BTreeSet::new();
                while let Some(v) = queue.pop() {
                    if !seen.insert(v) {
                        continue;
                    }
                    for wire in topology.connections.iter().filter(|w| w.destination == v) {
                        match wire.source {
                            Source::Junction(j) => queue.push(Vertex::Junction(j)),
                            Source::Exit(exit) => {
                                if regions.branches[first].contains(&Vertex::Node(exit.node))
                                    || (exit.node == NodeId::Block(block)
                                        && exit.branch == Some(first))
                                {
                                    approaches.insert(ports[&exit]);
                                }
                            }
                        }
                    }
                }
                let entry = vertices[&entry];
                for &approach in &approaches {
                    initial.edges.push((entry, approach, 0));
                }
                minima.push((entry, approaches.into_iter().collect()));
            }
            for group in &regions.groups {
                for sibling in regions.later_siblings(group) {
                    for left in regions.reserved(group, sibling) {
                        for right in regions.outside(group, sibling) {
                            initial.before(vertices[&left], vertices[&right]);
                        }
                    }
                }
            }
        }
    }

    fn coordinate(&self, item: Active, constraints: &mut Inequalities) -> usize {
        let variable = constraints.fresh();
        if let Active::Back(i) = item {
            constraints
                .edges
                .extend([(self.spines[i], variable, 0), (variable, self.upper[i], 0)]);
        }
        variable
    }

    fn departure(&self, wire: Connection) -> usize {
        if let Some(case) =
            crate::construct::choice::case_destination(wire.source, wire.destination)
        {
            self.vertices[&Vertex::Node(case)]
        } else {
            match wire.source {
                Source::Exit(exit) => self.ports[&exit],
                Source::Junction(junction) => self.vertices[&Vertex::Junction(junction)],
            }
        }
    }

    fn enumerate(
        &mut self,
        events: &mut Vec<Event>,
        cut: &[Active],
        sides: &[Option<Side>],
        constraints: &Inequalities,
    ) -> Option<Arrangement> {
        self.visited += 1;
        if self.quick && self.visited > 512 {
            return None;
        }
        assert!(
            self.visited < STATES,
            "the independent space exceeded its test bound"
        );
        constraints.solve()?;
        if events
            .iter()
            .filter_map(|e| e.vertex)
            .map(|v| self.event_vertices(v).len())
            .sum::<usize>()
            == self.vertices.len()
        {
            let coordinates = self.minimum(constraints, 0)?;
            let built = self.draw(events, sides, &coordinates);
            verify::arrangement(self.flow, self.topology, &built).unwrap_or_else(|reason| {
                panic!("the reference built an invalid witness: {reason}")
            });
            return Some(built);
        }
        // Independently enumerate linear extensions of the cut's nonsharing
        // pairs. Bubble each extension into place through allowed common rails.
        for order in self.orders(cut) {
            let kept = events.len();
            let mut next = constraints.clone();
            let mut current = cut.to_vec();
            for (position, item) in order.iter().enumerate() {
                let mut at = current.iter().position(|a| a == item).unwrap();
                while at > position {
                    let before = current
                        .iter()
                        .map(|&a| {
                            let variable = self.coordinate(a, &mut next);
                            (a, variable)
                        })
                        .collect::<BTreeMap<_, _>>();
                    for pair in current.windows(2) {
                        next.before(before[&pair[0]], before[&pair[1]]);
                    }
                    let input = current[at - 1..=at].to_vec();
                    let mut after = before.clone();
                    after.insert(input[0], before[&input[1]]);
                    after.insert(input[1], before[&input[0]]);
                    current.swap(at - 1, at);
                    events.push(Event {
                        vertex: None,
                        before,
                        after,
                        input,
                        output: current[at - 1..=at].to_vec(),
                    });
                    at -= 1;
                }
            }
            let found = self.vertices_at_cut(events, &current, sides, &next);
            events.truncate(kept);
            if found.is_some() {
                return found;
            }
        }
        None
    }

    fn orders(&self, cut: &[Active]) -> Vec<Vec<Active>> {
        if !self.exchanges {
            return vec![cut.to_vec()];
        }
        let predecessors = cut
            .iter()
            .enumerate()
            .map(|(i, &right)| {
                cut[..i]
                    .iter()
                    .enumerate()
                    .filter_map(|(j, &left)| {
                        let shared = if let (Active::Edge(a), Active::Edge(b)) = (left, right) {
                            let (a, b) =
                                (self.topology.connections[a], self.topology.connections[b]);
                            (a.source == b.source
                                && crate::construct::choice::case_destination(
                                    a.source,
                                    a.destination,
                                )
                                .is_none())
                                || a.destination == b.destination
                        } else {
                            false
                        };
                        (!shared).then_some(j)
                    })
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>();
        let mut result = Vec::new();
        extensions(&predecessors, &mut Vec::new(), &mut result);
        result
            .into_iter()
            .map(|order| order.into_iter().map(|i| cut[i]).collect())
            .collect()
    }

    fn candidate(
        &self,
        vertex: Vertex,
        events: &[Event],
        cut: &[Active],
        sides: &[Option<Side>],
    ) -> Option<Candidate> {
        if matches!(vertex, Vertex::Node(NodeId::Case { branch, .. }) if branch > 0)
            || events.iter().any(|event| event.vertex == Some(vertex))
        {
            return None;
        }
        let members = self.event_vertices(vertex);
        if self
            .topology
            .connections
            .iter()
            .chain(&self.topology.order)
            .filter(|w| members.contains(&w.destination))
            .any(|w| {
                !events
                    .iter()
                    .filter_map(|e| e.vertex)
                    .any(|v| self.event_vertices(v).contains(&Vertex::from(w.source)))
            })
        {
            return None;
        }
        let entry = self
            .topology
            .loops
            .iter()
            .position(|l| vertex == Vertex::Junction(l.entry));
        let tail = self
            .topology
            .loops
            .iter()
            .position(|l| vertex == Vertex::Junction(l.tail));
        if tail.is_some_and(|i| sides[i].is_none()) {
            return None;
        }
        let mut input = self
            .topology
            .connections
            .iter()
            .enumerate()
            .filter(|(_, w)| members.contains(&w.destination))
            .map(|(i, _)| Active::Edge(i))
            .collect::<BTreeSet<_>>();
        if let Some(i) = tail {
            input.insert(Active::Back(i));
        }
        let position = if input.is_empty() {
            if !cut.is_empty() {
                return None;
            }
            0
        } else {
            let position = cut.iter().position(|a| input.contains(a))?;
            if position + input.len() > cut.len()
                || !cut[position..position + input.len()]
                    .iter()
                    .all(|a| input.contains(a))
            {
                return None;
            }
            position
        };
        let input = cut[position..position + input.len()].to_vec();
        if members.len() > 1 {
            let destinations = input
                .iter()
                .map(|a| match a {
                    Active::Edge(w) => self.topology.connections[*w].destination,
                    Active::Back(_) => unreachable!("a case consumes no back edge"),
                })
                .collect::<Vec<_>>();
            if destinations != members {
                return None;
            }
        }
        if let Some(i) = tail
            && match sides[i].unwrap() {
                Side::Left => input.first(),
                Side::Right => input.last(),
            } != Some(&Active::Back(i))
        {
            return None;
        }
        let emissions = self.emissions(vertex);
        Some((entry, tail, position, input, emissions))
    }

    fn emissions(&self, vertex: Vertex) -> Vec<Vec<Active>> {
        let mut departures = BTreeMap::<Source, Vec<Active>>::new();
        let members = self.event_vertices(vertex);
        for (i, wire) in self.topology.connections.iter().enumerate() {
            if members.contains(&Vertex::from(wire.source)) {
                departures
                    .entry(wire.source)
                    .or_default()
                    .push(Active::Edge(i));
            }
        }
        let mut emissions = vec![Vec::new()];
        for group in departures.values() {
            let mut options = if group.iter().all(|item| {
                let Active::Edge(wire) = item else {
                    return false;
                };
                let connection = self.topology.connections[*wire];
                crate::construct::choice::case_destination(
                    connection.source,
                    connection.destination,
                )
                .is_some()
            }) {
                vec![group.clone()]
            } else {
                Vec::new()
            };
            if options.is_empty() {
                permute(group.clone(), 0, &mut options);
            }
            let mut next = Vec::new();
            for prefix in emissions {
                for option in &options {
                    next.push([prefix.as_slice(), option.as_slice()].concat());
                }
            }
            emissions = next;
        }
        // Prefer placing paths to each downstream merge together; this is
        // only the order of alternatives, never a contiguity restriction.
        emissions.sort_by_key(|items| {
            items
                .iter()
                .map(|item| {
                    let Active::Edge(w) = item else {
                        unreachable!()
                    };
                    &self.order_keys[&self.topology.connections[*w].destination]
                })
                .collect::<Vec<_>>()
        });
        if self.quick {
            let choice = (self.variation / self.divisors[&vertex]) % emissions.len();
            emissions = vec![emissions.swap_remove(choice)];
        }
        emissions
    }

    /// Enumerated independently of the production event table. A whole case
    /// row is indivisible; other vertices each have their own event.
    fn event_vertices(&self, vertex: Vertex) -> Vec<Vertex> {
        if let Vertex::Node(NodeId::Case { choice, .. }) = vertex {
            self.vertices
                .keys()
                .copied()
                .filter(|v| {
                    matches!(
                        v,
                        Vertex::Node(NodeId::Case { choice: c, .. }) if *c == choice
                    )
                })
                .collect()
        } else {
            vec![vertex]
        }
    }

    fn vertices_at_cut(
        &mut self,
        events: &mut Vec<Event>,
        cut: &[Active],
        sides: &[Option<Side>],
        constraints: &Inequalities,
    ) -> Option<Arrangement> {
        for vertex in self.priority.clone() {
            let Some((entry, tail, _position, input, emissions)) =
                self.candidate(vertex, events, cut, sides)
            else {
                continue;
            };
            let choices = if let Some(i) = entry {
                if self.quick {
                    vec![Some(
                        if (self.variation / 2usize.saturating_pow(i as u32)).is_multiple_of(2) {
                            Side::Left
                        } else {
                            Side::Right
                        },
                    )]
                } else {
                    vec![Some(Side::Left), Some(Side::Right)]
                }
            } else {
                vec![None]
            };
            for side in choices {
                for emitted in &emissions {
                    let mut next_sides = sides.to_vec();
                    let mut output = emitted.clone();
                    let mut next = constraints.clone();
                    if let Some(i) = entry {
                        next_sides[i] = side;
                        match side.unwrap() {
                            Side::Left => output.insert(0, Active::Back(i)),
                            Side::Right => output.push(Active::Back(i)),
                        }
                        self.outside(i, side.unwrap(), &mut next);
                    }
                    let (event, next_cut) = self.event(
                        vertex,
                        (entry, tail),
                        cut,
                        (&input, &output),
                        &next_sides,
                        &mut next,
                    );
                    events.push(event);
                    let found = self.enumerate(events, &next_cut, &next_sides, &next);
                    events.pop();
                    if found.is_some() {
                        return found;
                    }
                }
            }
        }
        None
    }

    fn event(
        &self,
        vertex: Vertex,
        loops: (Option<usize>, Option<usize>),
        cut: &[Active],
        incident: (&[Active], &[Active]),
        sides: &[Option<Side>],
        next: &mut Inequalities,
    ) -> (Event, Vec<Active>) {
        let (entry, tail) = loops;
        let (input, output) = incident;
        let position = cut.iter().position(|a| input.contains(a)).unwrap_or(0);
        let before = cut
            .iter()
            .map(|&item| (item, self.coordinate(item, next)))
            .collect::<BTreeMap<_, _>>();
        let mut after = before.clone();
        after.retain(|item, _| !input.contains(item));
        after.extend(
            output
                .iter()
                .map(|&item| (item, self.coordinate(item, next))),
        );
        let own = self.vertices[&vertex];
        let own_loop = entry.or(tail);
        let flank = own_loop.map(|i| sides[i].unwrap());
        let at_back_edge = own_loop.map(|i| {
            if entry.is_some() {
                after[&Active::Back(i)]
            } else {
                before[&Active::Back(i)]
            }
        });
        let mut center = Vec::new();
        if flank == Some(Side::Left) {
            center.push(at_back_edge.unwrap());
        }
        let arrivals = input
            .iter()
            .filter(|a| matches!(a, Active::Edge(_)))
            .map(|a| before[a]);
        let left_arrivals = tail.is_some() && flank == Some(Side::Right);
        if left_arrivals {
            center.extend(arrivals.clone());
        }
        center.push(own);
        if !left_arrivals {
            center.extend(arrivals);
        }
        let mut previous_port = None;
        for &item in output {
            if let Active::Edge(w) = item {
                let connection = self.topology.connections[w];
                let port = self.departure(connection);
                // The first port is equal to the node's column.
                if previous_port.is_some() && previous_port != Some(port) {
                    center.push(port);
                }
                center.push(after[&item]);
                previous_port = Some(port);
            }
        }
        if flank == Some(Side::Right) {
            center.push(at_back_edge.unwrap());
        }
        let members = self.event_vertices(vertex);
        if members.len() > 1 {
            center.clear();
            for member in members {
                let column = self.vertices[&member];
                center.push(column);
                for &item in input {
                    if let Active::Edge(w) = item
                        && self.topology.connections[w].destination == member
                    {
                        next.equal(before[&item], column);
                    }
                }
                for &item in output {
                    if let Active::Edge(w) = item
                        && Vertex::from(self.topology.connections[w].source) == member
                    {
                        center.push(after[&item]);
                    }
                }
            }
        }
        let row = cut[..position]
            .iter()
            .map(|a| before[a])
            .chain(center)
            .chain(cut[position + input.len()..].iter().map(|a| before[a]))
            .collect::<Vec<_>>();
        for pair in row.windows(2) {
            next.before(pair[0], pair[1]);
        }
        let next_cut = [&cut[..position], output, &cut[position + input.len()..]].concat();
        let event = Event {
            vertex: Some(vertex),
            before,
            after,
            input: input.to_vec(),
            output: output.to_vec(),
        };
        (event, next_cut)
    }

    fn outside(&self, i: usize, side: Side, next: &mut Inequalities) {
        let loop_ = self.topology.loops[i];
        let end = self.flow.blocks[loop_.header].loop_end.unwrap();
        let mut body =
            crate::construct::loop_block::body_vertices(self.flow, self.topology, loop_.header)
                .into_iter()
                .map(|v| self.vertices[&v])
                .collect::<Vec<_>>();
        body.extend(
            self.topology
                .loops
                .iter()
                .enumerate()
                .filter(|(_, l)| (loop_.header + 1..end).contains(&l.header))
                .map(|(j, _)| {
                    if side == Side::Left {
                        self.spines[j]
                    } else {
                        self.upper[j]
                    }
                }),
        );
        for coordinate in body {
            match side {
                Side::Left => next.before(self.upper[i], coordinate),
                Side::Right => next.before(coordinate, self.spines[i]),
            }
        }
    }

    fn minimum(&self, constraints: &Inequalities, at: usize) -> Option<Vec<i32>> {
        let Some((entry, options)) = self.minima.get(at) else {
            return constraints.solve();
        };
        for &option in options {
            let mut next = constraints.clone();
            next.equal(*entry, option);
            if next.solve().is_some()
                && let Some(solution) = self.minimum(&next, at + 1)
            {
                return Some(solution);
            }
        }
        None
    }

    fn draw(&self, events: &[Event], sides: &[Option<Side>], x: &[i32]) -> Arrangement {
        let mut drawing = Arrangement {
            rank: events
                .iter()
                .enumerate()
                .flat_map(|(i, e)| {
                    e.vertex
                        .into_iter()
                        .flat_map(move |v| self.event_vertices(v).into_iter().map(move |v| (v, i)))
                })
                .collect(),
            ranks: events.len(),
            column: self.vertices.iter().map(|(&v, &i)| (v, x[i])).collect(),
            exit_offset: self
                .ports
                .iter()
                .map(|(&e, &i)| (e, x[i] - x[self.vertices[&Vertex::Node(e.node)]]))
                .collect(),
            routes: self
                .topology
                .connections
                .iter()
                .map(|wire| {
                    let arrival = x[self.vertices[&wire.destination]];
                    Route {
                        departure: x[self.departure(*wire)],
                        arrival,
                        runs: Vec::new(),
                    }
                })
                .collect(),
            gap_lanes: vec![0; events.len()],
            back_routes: self
                .topology
                .loops
                .iter()
                .enumerate()
                .map(|(i, loop_)| {
                    let entry = events
                        .iter()
                        .find(|e| e.vertex == Some(Vertex::Junction(loop_.entry)))
                        .unwrap();
                    let tail = events
                        .iter()
                        .find(|e| e.vertex == Some(Vertex::Junction(loop_.tail)))
                        .unwrap();
                    (
                        i,
                        Route {
                            departure: x[entry.after[&Active::Back(i)]],
                            arrival: x[tail.before[&Active::Back(i)]],
                            runs: Vec::new(),
                        },
                    )
                })
                .collect(),
            contours: self
                .topology
                .loops
                .iter()
                .enumerate()
                .map(|(i, loop_)| {
                    let entry = events
                        .iter()
                        .find(|e| e.vertex == Some(Vertex::Junction(loop_.entry)))
                        .unwrap();
                    Contour {
                        side: sides[i].unwrap(),
                        column: x[entry.after[&Active::Back(i)]],
                        lane: 0,
                    }
                })
                .collect(),
        };
        Self::draw_strips(events, x, &mut drawing);
        drawing
            .back_routes
            .retain(|_, route| !route.runs.is_empty());
        drawing
    }
    fn draw_strips(events: &[Event], x: &[i32], drawing: &mut Arrangement) {
        for (gap, pair) in events.windows(2).enumerate() {
            let (above, below) = (&pair[0], &pair[1]);
            let mut current = above
                .after
                .iter()
                .map(|(&wire, &variable)| (wire, x[variable]))
                .collect::<BTreeMap<_, _>>();
            for &item in above.output.iter().filter(|_| above.vertex.is_some()) {
                if let Active::Edge(w) = item {
                    let route = &mut drawing.routes[w];
                    if route.departure != current[&item] {
                        route.runs.push(Run {
                            gap,
                            lane: 0,
                            enter: route.departure,
                            exit: current[&item],
                        });
                    }
                }
            }
            let target = below
                .before
                .iter()
                .map(|(&wire, &variable)| (wire, x[variable]))
                .collect::<BTreeMap<_, _>>();
            let mut lane = 1;
            while current != target {
                let movable = current
                    .iter()
                    .find_map(|(&item, &from)| {
                        let to = target[&item];
                        (from != to
                            && !current.iter().any(|(&other, &at)| {
                                other != item && at >= from.min(to) && at <= from.max(to)
                            }))
                        .then_some((item, from, to))
                    })
                    .expect("ordered cuts admit an unobstructed horizontal move");
                let (item, from, to) = movable;
                let route = match item {
                    Active::Edge(w) => &mut drawing.routes[w],
                    Active::Back(i) => drawing.back_routes.get_mut(&i).unwrap(),
                };
                route.runs.push(Run {
                    gap,
                    lane,
                    enter: from,
                    exit: to,
                });
                current.insert(item, to);
                lane += 1;
            }
            for &item in &below.input {
                if let Active::Edge(w) = item {
                    let route = &mut drawing.routes[w];
                    let enter = current[&item];
                    let arrival = if below.vertex.is_some() {
                        route.arrival
                    } else {
                        let other = below.input.iter().find(|&&a| a != item).unwrap();
                        current[other]
                    };
                    if enter != arrival {
                        route.runs.push(Run {
                            gap,
                            lane,
                            enter,
                            exit: arrival,
                        });
                    }
                }
            }
            drawing.gap_lanes[gap] = lane + 1;
        }
    }
}

fn permute(mut values: Vec<Active>, position: usize, result: &mut Vec<Vec<Active>>) {
    if position == values.len() {
        result.push(values);
        return;
    }
    for i in position..values.len() {
        values.swap(position, i);
        permute(values.clone(), position + 1, result);
        values.swap(position, i);
    }
}

fn extensions(predecessors: &[Vec<usize>], prefix: &mut Vec<usize>, result: &mut Vec<Vec<usize>>) {
    if prefix.len() == predecessors.len() {
        result.push(prefix.clone());
        return;
    }
    for (v, incoming) in predecessors.iter().enumerate() {
        if !prefix.contains(&v) && incoming.iter().all(|i| prefix.contains(i)) {
            prefix.push(v);
            extensions(predecessors, prefix, result);
            prefix.pop();
        }
    }
}

type Candidate = (
    Option<usize>,
    Option<usize>,
    usize,
    Vec<Active>,
    Vec<Vec<Active>>,
);

fn vertex_priority(topology: &Topology) -> (Vec<Vertex>, BTreeMap<Vertex, Vec<bool>>) {
    let mut reach = topology
        .vertices
        .iter()
        .map(|&v| (v, BTreeSet::new()))
        .collect::<BTreeMap<_, _>>();
    for wire in topology.connections.iter().chain(&topology.order) {
        reach
            .get_mut(&Vertex::from(wire.source))
            .unwrap()
            .insert(wire.destination);
    }
    for l in &topology.loops {
        reach
            .get_mut(&Vertex::Junction(l.entry))
            .unwrap()
            .insert(Vertex::Junction(l.tail));
    }
    loop {
        let previous = reach.clone();
        for children in reach.values_mut() {
            let more = children
                .iter()
                .flat_map(|v| previous[v].iter().copied())
                .collect::<Vec<_>>();
            children.extend(more);
        }
        if reach == previous {
            break;
        }
    }
    let mut vertices = topology.vertices.clone();
    vertices.sort_by_key(|v| std::cmp::Reverse(reach[v].len()));
    let merges = topology
        .vertices
        .iter()
        .copied()
        .filter(|v| {
            topology
                .connections
                .iter()
                .filter(|w| w.destination == *v)
                .count()
                > 1
                || topology
                    .loops
                    .iter()
                    .any(|l| *v == Vertex::Junction(l.tail))
        })
        .collect::<Vec<_>>();
    let keys = reach
        .into_iter()
        .map(|(v, reached)| {
            (
                v,
                merges
                    .iter()
                    .map(|m| v == *m || reached.contains(m))
                    .collect(),
            )
        })
        .collect();
    (vertices, keys)
}
