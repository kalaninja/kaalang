//! Exact connection union and per-execution reduction over conditions.

use std::collections::{BTreeMap, BTreeSet};

use super::{
    Executions,
    condition::{ALWAYS, Condition, NEVER},
};
use crate::{
    BlockKind, Execution, Flow, ProducerId,
    topology::{
        Analyzed, Connection, Cycle, CycleBoundary, Destination, ExitId, NodeId, Source, Topology,
        Vertex,
    },
};

impl Executions {
    /// Concrete witnesses for the union facts read outside connection reduction.
    pub(crate) fn observations(&mut self) -> Vec<Execution> {
        let conditions: Vec<_> = self
            .runs
            .iter()
            .chain(self.produced.values())
            .chain(self.dependencies.values())
            .chain(self.outcomes.values())
            .copied()
            .collect();
        conditions
            .into_iter()
            .filter_map(|when| {
                let when = self.conditions.and(self.domain, when);
                self.conditions
                    .witness(when)
                    .map(|assignment| self.expression(&assignment))
            })
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect()
    }

    fn departures(
        &mut self,
        model: &Analyzed<'_>,
        boundaries: &[CycleBoundary],
        structural: &BTreeMap<usize, usize>,
        block: usize,
    ) -> Vec<(Source, Condition)> {
        if let Some(&junction) = structural.get(&block) {
            return vec![(Source::Junction(junction), self.runs[block])];
        }
        let declaration = &model.flow.blocks[block];
        if declaration.branch_count() > 0 {
            let mut exits: Vec<_> = (0..declaration.outputs.len())
                .map(|output| {
                    (
                        crate::topology::source(
                            model,
                            ProducerId::BlockOutput { block, output },
                            boundaries,
                        ),
                        self.produced(ProducerId::BlockOutput { block, output }),
                    )
                })
                .collect();
            if declaration.kind == BlockKind::Cycle {
                let absent = self
                    .conditions
                    .minus(self.runs[block], self.selected_runs[block]);
                exits[0].1 = self.conditions.or(exits[0].1, absent);
            }
            exits
        } else {
            vec![(
                crate::topology::source(
                    model,
                    ProducerId::BlockOutput { block, output: 0 },
                    boundaries,
                ),
                self.runs[block],
            )]
        }
    }

    #[allow(clippy::too_many_lines, clippy::too_many_arguments)] // The same projection context and conditional sources as ordinary connections.
    pub(crate) fn connections(
        &mut self,
        model: &Analyzed<'_>,
        groups: &[Vec<usize>],
        cycles: &[Cycle],
        boundaries: &[CycleBoundary],
        structural: &BTreeMap<usize, usize>,
        vertices: &[Vertex],
    ) -> Vec<Connection> {
        let flow = model.flow;
        let mut edges = BTreeMap::new();
        let mut add = |this: &mut Self, source, destination, when| {
            let entry = edges
                .entry(Connection {
                    source,
                    destination,
                })
                .or_insert(NEVER);
            *entry = this.conditions.or(*entry, when);
        };
        let end = flow.blocks.len() - 1;
        let terminal = self.outcomes.iter().filter(|(outcome, _)| matches!(outcome, crate::ExecutionOutcome::Return { block_index } if flow.blocks[*block_index].transition_target.is_none())).fold(NEVER, |sum, (_, &when)| self.conditions.or(sum, when));
        let runs = |this: &Self, block| {
            if block == end {
                terminal
            } else {
                this.runs[block]
            }
        };
        let junction_of = |wire: &proc_macro2::Ident| {
            groups
                .iter()
                .position(|group| group.iter().any(|&merge| model.merges[merge].wire == *wire))
        };
        let mut order = Vec::new();
        crate::plan::serial_order(model.execution_plan, &mut order);
        order.retain(|&block| {
            crate::topology::represented(model, structural, block)
                || (!model.collapse_cycles
                    && matches!(
                        flow.blocks[block].kind,
                        BlockKind::Export | BlockKind::Continue
                    )
                    && !structural.contains_key(&block))
        });
        let mut previous = BTreeMap::from([(Source::Exit(ExitId::of(NodeId::Start)), ALWAYS)]);
        for (position, &block) in order.iter().enumerate() {
            let participates = runs(self, block);
            let transfer = (!model.collapse_cycles)
                .then(|| crate::topology::transfer_junction(model, cycles, boundaries, block))
                .flatten();
            let destination = match transfer {
                Some(junction) if !structural.contains_key(&block) => {
                    Destination::Junction(junction)
                }
                _ => crate::topology::destination(structural, block),
            };
            for (&source, &when) in &previous {
                let edge = self.conditions.and(when, participates);
                add(self, source, destination, edge);
            }
            if block == end || flow.blocks[block].transition_target.is_some() {
                continue;
            }
            for when in previous.values_mut() {
                *when = self.conditions.minus(*when, participates);
            }
            if let Some(junction) = transfer {
                let entry = previous.entry(Source::Junction(junction)).or_insert(NEVER);
                *entry = self.conditions.or(*entry, participates);
                continue;
            }
            let mut next_when = participates;
            let mut nexts = Vec::new();
            for &next in &order[position + 1..] {
                let adjacent = self.conditions.and(next_when, runs(self, next));
                if adjacent != NEVER {
                    nexts.push((Some(next), adjacent));
                }
                next_when = self.conditions.minus(next_when, runs(self, next));
            }
            nexts.push((None, next_when));
            for (exit, departure) in self.departures(model, boundaries, structural, block) {
                for &(next, adjacent) in &nexts {
                    let mut remaining = self.conditions.and(departure, adjacent);
                    for (junction, group) in groups.iter().enumerate() {
                        let mut completed = NEVER;
                        for merge in group.iter().map(|&merge| &model.merges[merge]) {
                            if next.is_some_and(|next| merge.before.contains(&next))
                                || (!merge.before.contains(&block)
                                    && !merge.producers.iter().any(|&producer| {
                                        crate::topology::source(model, producer, boundaries) == exit
                                    }))
                            {
                                continue;
                            }
                            let provided = self.merged(merge);
                            completed = self.conditions.or(completed, provided);
                        }
                        let through = self.conditions.and(remaining, completed);
                        let entry = previous.entry(Source::Junction(junction)).or_insert(NEVER);
                        *entry = self.conditions.or(*entry, through);
                        remaining = self.conditions.minus(remaining, completed);
                    }
                    let entry = previous.entry(exit).or_insert(NEVER);
                    *entry = self.conditions.or(*entry, remaining);
                }
            }
        }
        for (dependency, when) in self.dependencies.clone() {
            let capture = dependency.capture;
            let returns = flow.blocks[capture.block].kind == BlockKind::Return;
            if !returns && !crate::topology::represented(model, structural, capture.block) {
                continue;
            }
            let entered = crate::topology::entered(model, &dependency);
            let source = entered.map_or_else(
                || crate::topology::source(model, dependency.producer, boundaries),
                |header| Source::Junction(structural[&header]),
            );
            let consumer = if returns {
                Destination::Node(NodeId::Block(
                    if flow.blocks[capture.block].transition_target.is_some() {
                        capture.block
                    } else {
                        end
                    },
                ))
            } else {
                crate::topology::destination(structural, capture.block)
            };
            let wire = &flow.blocks[capture.block].inputs[capture.input].ident;
            if let Some(junction) = junction_of(wire).filter(|_| entered.is_none()) {
                add(self, source, Destination::Junction(junction), when);
                add(self, Source::Junction(junction), consumer, when);
            } else {
                add(self, source, consumer, when);
            }
        }
        for (junction, group) in groups.iter().enumerate() {
            for merge in group.iter().map(|&index| &model.merges[index]) {
                let provided = self.merged(merge);
                for &producer in &merge.producers {
                    add(
                        self,
                        crate::topology::source(model, producer, boundaries),
                        Destination::Junction(junction),
                        self.produced(producer),
                    );
                }
                for &after in &merge.after {
                    if crate::topology::represented(model, structural, after) {
                        let when = self.conditions.and(provided, runs(self, after));
                        add(
                            self,
                            Source::Junction(junction),
                            crate::topology::destination(structural, after),
                            when,
                        );
                    }
                }
                for &before in &merge.before {
                    if crate::topology::represented(model, structural, before) {
                        for (exit, departure) in
                            self.departures(model, boundaries, structural, before)
                        {
                            let when = self.conditions.and(provided, departure);
                            add(self, exit, Destination::Junction(junction), when);
                        }
                    }
                }
            }
        }
        for &block in &self.selectors.clone() {
            if flow.blocks[block].kind == BlockKind::Choice
                && crate::topology::represented_block(model, block)
            {
                for branch in 0..flow.blocks[block].outputs.len() {
                    add(
                        self,
                        Source::Exit(ExitId::of(NodeId::Block(block))),
                        Destination::Node(NodeId::Case {
                            choice: block,
                            branch,
                        }),
                        self.produced(ProducerId::BlockOutput {
                            block,
                            output: branch,
                        }),
                    );
                }
            }
        }
        self.reduce(&edges, vertices)
    }

    pub(crate) fn order_exits(
        &mut self,
        model: &Analyzed<'_>,
        structural: &BTreeMap<usize, usize>,
        topology: &mut Topology,
    ) {
        for (block, declaration) in model.flow.blocks.iter().enumerate() {
            let Some(target) = declaration.export_target else {
                continue;
            };
            let mut next_when = self.runs[block];
            let end = model.flow.blocks[target]
                .cycle_end
                .expect("a cycle owns a body");
            for next in block + 1..model.flow.blocks.len() - 1 {
                if !crate::topology::represented(model, structural, next) {
                    continue;
                }
                let adjacent = self.conditions.and(next_when, self.runs[next]);
                if self.has(adjacent) {
                    for cycle in topology
                        .cycles
                        .iter()
                        .filter(|cycle| (target..end).contains(&cycle.header))
                    {
                        topology.order.push(Connection {
                            source: Source::Junction(cycle.tail),
                            destination: crate::topology::destination(structural, next),
                        });
                    }
                }
                next_when = self.conditions.minus(next_when, self.runs[next]);
            }
        }
    }

    pub(crate) fn prefer_left(&mut self, flow: &Flow, header: usize) -> bool {
        let end = flow.blocks[header].cycle_end.expect("a cycle owns a body");
        let Some(first) = (header + 1..end).find(|&block| flow.draws_branches(block)) else {
            return true;
        };
        let selected = self.selected(first, flow.blocks[first].branch_count() - 1);
        let repeated = self
            .outcomes
            .get(&crate::ExecutionOutcome::Repeat {
                cycle_index: header,
            })
            .copied()
            .unwrap_or(NEVER);
        let left = self.conditions.minus(repeated, selected);
        self.has(left)
    }

    fn reduce(
        &mut self,
        edges: &BTreeMap<Connection, Condition>,
        vertices: &[Vertex],
    ) -> Vec<Connection> {
        fn visit(
            vertex: usize,
            successors: &[BTreeMap<usize, Condition>],
            seen: &mut [bool],
            order: &mut Vec<usize>,
        ) {
            if seen[vertex] {
                return;
            }
            seen[vertex] = true;
            for &next in successors[vertex].keys() {
                visit(next, successors, seen, order);
            }
            order.push(vertex);
        }
        let count = vertices.len();
        let index = |vertex| {
            vertices
                .binary_search(&vertex)
                .expect("a connection endpoint is a vertex")
        };
        let mut successors = vec![BTreeMap::new(); count];
        for (&connection, &when) in edges {
            if !self.has(when) {
                continue;
            }
            let entry = successors[index(Vertex::from(connection.source))]
                .entry(index(connection.destination))
                .or_insert(NEVER);
            *entry = self.conditions.or(*entry, when);
        }
        let mut order = Vec::new();
        let mut seen = vec![false; count];
        for vertex in 0..count {
            visit(vertex, &successors, &mut seen, &mut order);
        }
        let mut reach = vec![vec![NEVER; count]; count];
        for vertex in order {
            for (&next, &edge) in &successors[vertex] {
                reach[vertex][next] = self.conditions.or(reach[vertex][next], edge);
                let (lower, upper) = reach.split_at_mut(vertex.max(next));
                let (row, inherited) = if vertex < next {
                    (&mut lower[vertex], &upper[0])
                } else {
                    (&mut upper[0], &lower[next])
                };
                for (destination, &reachable) in row.iter_mut().zip(inherited) {
                    if reachable != NEVER {
                        let through = self.conditions.and(edge, reachable);
                        *destination = self.conditions.or(*destination, through);
                    }
                }
            }
        }
        edges
            .iter()
            .filter_map(|(&connection, &when)| {
                let from = index(Vertex::from(connection.source));
                let to = index(connection.destination);
                let mut alternate = NEVER;
                for (&middle, &edge) in &successors[from] {
                    if middle == to || reach[middle][to] == NEVER {
                        continue;
                    }
                    let path = self.conditions.and(edge, reach[middle][to]);
                    alternate = self.conditions.or(alternate, path);
                }
                let direct = self.conditions.minus(when, alternate);
                self.has(direct).then_some(connection)
            })
            .collect()
    }
}
