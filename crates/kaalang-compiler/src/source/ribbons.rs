//! The ribbon of each drawn connection, whose wires the diagram does not draw
//! one by one: a fact a renderer reports beside its drawing.

use std::collections::{BTreeMap, BTreeSet};

use proc_macro2::Ident;

#[cfg(doc)]
use crate::topology::Topology;
use crate::{
    BlockKind, ProducerId, SemanticModel,
    topology::{Connection, ExitId, NodeId, NodeKind, Source, Vertex},
};

/// The ribbon of each drawn connection and junction of one local flow: the
/// wires in scope there that a block further along captures. A wire stands as
/// its producer; once alternative producers meet, the first of them stands for
/// the merged wire. A connection that carries different wires in different
/// executions carries them all.
pub struct Ribbons {
    /// Per connection, in [`Topology::connections`] order, in source order.
    pub connections: Vec<Vec<ProducerId>>,
    /// Per back edge, in [`Topology::back_edges`] order, in source order.
    pub back_edges: Vec<Vec<ProducerId>>,
    /// Per junction, in [`Topology::junctions`] order, in source order: what
    /// reaches it and is captured there or further along.
    pub junctions: Vec<Vec<ProducerId>>,
}

impl SemanticModel {
    /// The ribbon of each connection and junction this model draws.
    ///
    /// A wire rides from the exit that provides it towards every block that
    /// captures it, as far as the last such capture; a borrow does not end it.
    /// So one provided before a split reaches every branch that captures it,
    /// and one provided inside a branch passes a merge only when it merges
    /// there. A back edge carries the outer wires captured further along, in
    /// the next iteration or after the cycle, never an iteration's own.
    #[must_use]
    pub fn ribbons(&self) -> Ribbons {
        let topology = &self.topology;
        let connections = topology.connections.len();
        // A for cycle draws neither how its for-end leads back to its
        // for-entry for the next item, nor how its for-entry, with no item
        // left, leads on to its for-end, the only way there when every route
        // through the body diverges.
        let undrawn: Vec<Connection> = topology
            .cycle_boundaries
            .iter()
            .filter_map(|boundary| boundary.caps)
            .flat_map(|caps| {
                let link = |from, to| Connection {
                    source: Source::Exit(ExitId::of(from)),
                    destination: Vertex::Node(to),
                };
                [link(caps.bottom, caps.top), link(caps.top, caps.bottom)]
            })
            .collect();
        let edges: Vec<&Connection> = topology
            .connections
            .iter()
            .chain(&topology.back_edges)
            .chain(&undrawn)
            .collect();
        let numbered = Numbered::new(self, &edges);
        let (before, after) = neighbours(&edges);
        let forward = forward(&before, &after, connections);
        let backward: Vec<usize> = forward.iter().rev().copied().collect();

        // Which producers reach each edge, a wire provided again replacing them.
        let mut reached = vec![vec![0; numbered.producer_words]; edges.len()];
        settle(&mut reached, &forward, |edge, reached| {
            let mut bits = vec![0; numbered.producer_words];
            for &earlier in &before[edge] {
                or(&mut bits, &reached[earlier]);
            }
            let replaced = &numbered.replaced[edge];
            for ((bit, &gone), &new) in bits.iter_mut().zip(replaced).zip(&numbered.provides[edge])
            {
                *bit = *bit & !gone | new;
            }
            bits
        });

        // Which wires a block further along each edge captures, before any
        // exit provides them again.
        let mut wanted = vec![vec![0; numbered.wire_words]; edges.len()];
        settle(&mut wanted, &backward, |edge, wanted| {
            let mut bits = numbered.captures[edge].clone();
            for &later in &after[edge] {
                let again = &numbered.provides_wires[later];
                for ((bit, &further), &again) in bits.iter_mut().zip(&wanted[later]).zip(again) {
                    *bit |= further & !again;
                }
            }
            bits
        });

        // A junction carries what the edges entering it carry.
        let mut entering = vec![
            (
                vec![0; numbered.producer_words],
                vec![0; numbered.wire_words]
            );
            topology.junctions.len()
        ];
        for (edge, connection) in edges.iter().enumerate() {
            if let Vertex::Junction(junction) = connection.destination {
                or(&mut entering[junction].0, &reached[edge]);
                or(&mut entering[junction].1, &wanted[edge]);
            }
        }
        let mut ribbons = reached
            .iter()
            .zip(&wanted)
            .map(|(reached, wanted)| numbered.ribbon(reached, wanted));
        Ribbons {
            connections: ribbons.by_ref().take(connections).collect(),
            back_edges: ribbons.take(topology.back_edges.len()).collect(),
            junctions: entering
                .iter()
                .map(|(reached, wanted)| numbered.ribbon(reached, wanted))
                .collect(),
        }
    }
}

/// The producers and wires of one local flow numbered for bit sets, with what
/// each edge provides and what is captured at its end.
struct Numbered {
    /// Every producer an exit provides, in source order.
    producers: Vec<ProducerId>,
    /// Each producer's wire, by index.
    wire: Vec<usize>,
    producer_words: usize,
    wire_words: usize,
    /// Per edge, the producers its exit provides, as producers and as wires.
    provides: Vec<Bits>,
    provides_wires: Vec<Bits>,
    /// Per edge, the producers of the wires it provides again.
    replaced: Vec<Bits>,
    /// Per edge, the wires captured where it ends.
    captures: Vec<Bits>,
}

impl Numbered {
    fn new(model: &SemanticModel, edges: &[&Connection]) -> Self {
        let flow = &model.analysis.flow;
        let topology = &model.topology;
        let connections = topology.connections.len();
        // An expanded cycle provides each of its outputs where it completes.
        let results: Vec<(Source, ProducerId)> = topology
            .cycle_boundaries
            .iter()
            .flat_map(|boundary| {
                boundary
                    .results
                    .iter()
                    .enumerate()
                    .map(|(output, &result)| {
                        let block = boundary.header;
                        (result, ProducerId::BlockOutput { block, output })
                    })
            })
            .collect();
        let producers: Vec<ProducerId> = topology
            .exits
            .iter()
            .flat_map(|exit| exit.provides.iter().copied())
            .chain(results.iter().map(|&(_, producer)| producer))
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect();
        let mut indices = BTreeMap::<&Ident, usize>::new();
        let wire: Vec<usize> = producers
            .iter()
            .map(|&producer| {
                let next = indices.len();
                *indices.entry(flow.wire(producer)).or_insert(next)
            })
            .collect();
        let (producer_words, wire_words) = (words(producers.len()), words(indices.len()));
        let mut provides = vec![vec![0; producer_words]; edges.len()];
        let mut provides_wires = vec![vec![0; wire_words]; edges.len()];
        // Only a drawn forward connection provides: a back edge returns to
        // where an iteration starts, and a for cycle's completion provides at
        // its for-end.
        for (index, edge) in edges[..connections].iter().enumerate() {
            let from_exit = match edge.source {
                Source::Exit(exit) => topology.exit(exit).provides.as_slice(),
                Source::Junction(_) => &[],
            };
            let from_result = results
                .iter()
                .filter(|&&(result, _)| result == edge.source)
                .map(|(_, producer)| producer);
            for producer in from_exit.iter().chain(from_result) {
                let position = producers
                    .binary_search(producer)
                    .expect("every provided producer is listed");
                set(&mut provides[index], position);
                set(&mut provides_wires[index], wire[position]);
            }
        }
        let replaced = provides_wires
            .iter()
            .map(|wires| {
                let mut replaced = vec![0; producer_words];
                for (position, &of) in wire.iter().enumerate() {
                    if has(wires, of) {
                        set(&mut replaced, position);
                    }
                }
                replaced
            })
            .collect();
        let bits = |wires: &BTreeSet<&Ident>| {
            let mut bits = vec![0; wire_words];
            for &index in wires.iter().filter_map(|&wire| indices.get(wire)) {
                set(&mut bits, index);
            }
            bits
        };
        let (at, on_entry) = captured(model);
        let captures = edges
            .iter()
            .enumerate()
            .map(|(index, edge)| {
                let mut captures = vec![0; wire_words];
                let entered = (index < connections)
                    .then(|| on_entry.get(&edge.destination))
                    .flatten();
                for wires in at.get(&edge.destination).into_iter().chain(entered) {
                    or(&mut captures, &bits(wires));
                }
                captures
            })
            .collect();
        Self {
            producers,
            wire,
            producer_words,
            wire_words,
            provides,
            provides_wires,
            replaced,
            captures,
        }
    }

    /// The producers in `reached` whose wires are in `wanted`, one per wire:
    /// the first, which stands for a merged wire.
    fn ribbon(&self, reached: &[u64], wanted: &[u64]) -> Vec<ProducerId> {
        let mut seen = vec![0; self.wire_words];
        let mut ribbon = Vec::new();
        for (position, &producer) in self.producers.iter().enumerate() {
            let of = self.wire[position];
            if has(reached, position) && has(wanted, of) && !has(&seen, of) {
                set(&mut seen, of);
                ribbon.push(producer);
            }
        }
        ribbon
    }
}

/// Per edge, the edges that enter the vertex it leaves and those that leave the
/// vertex it enters.
fn neighbours(edges: &[&Connection]) -> (Vec<Vec<usize>>, Vec<Vec<usize>>) {
    let mut leaving = BTreeMap::<Vertex, Vec<usize>>::new();
    let mut entering = BTreeMap::<Vertex, Vec<usize>>::new();
    for (index, edge) in edges.iter().enumerate() {
        leaving
            .entry(Vertex::from(edge.source))
            .or_default()
            .push(index);
        entering.entry(edge.destination).or_default().push(index);
    }
    let at =
        |map: &BTreeMap<Vertex, Vec<usize>>, vertex| map.get(&vertex).cloned().unwrap_or_default();
    (
        edges
            .iter()
            .map(|edge| at(&entering, Vertex::from(edge.source)))
            .collect(),
        edges
            .iter()
            .map(|edge| at(&leaving, edge.destination))
            .collect(),
    )
}

/// A set of indices, one bit each.
type Bits = Vec<u64>;

fn words(indices: usize) -> usize {
    indices.div_ceil(64)
}

fn set(bits: &mut Bits, index: usize) {
    bits[index / 64] |= 1 << (index % 64);
}

fn has(bits: &[u64], index: usize) -> bool {
    bits[index / 64] >> (index % 64) & 1 == 1
}

fn or(into: &mut [u64], from: &[u64]) {
    for (word, &other) in into.iter_mut().zip(from) {
        *word |= other;
    }
}

/// Wires captured at each vertex.
type Captures<'a> = BTreeMap<Vertex, BTreeSet<&'a Ident>>;

/// The wires each vertex captures, borrowed or not, from every edge that
/// enters it, and those it captures only from an edge that enters a cycle.
///
/// A node captures its block's, end the authored `return`'s, a loop cycle's
/// tail its `continue`'s, and where an expanded cycle completes, unless its
/// exit provides them itself, the wires that become its outputs; none of these
/// transfers draws a node of its own. A collapsed cycle's block lists the wires
/// its body captures. An expanded cycle captures its gate, or a for cycle its
/// header captures, once, as the sequence enters it.
fn captured(model: &SemanticModel) -> (Captures<'_>, Captures<'_>) {
    let flow = &model.analysis.flow;
    let topology = &model.topology;
    let mut captured = Captures::new();
    let end = topology
        .nodes
        .iter()
        .find(|node| node.kind == NodeKind::End)
        .map(|node| Vertex::Node(node.id));
    for node in &topology.nodes {
        if let NodeId::Block(block) = node.id {
            captured
                .entry(Vertex::Node(node.id))
                .or_default()
                .extend(flow.blocks[block].inputs.iter().map(|input| &input.ident));
        }
    }
    for (index, block) in flow.blocks.iter().enumerate() {
        let provides = |result: &Source| match *result {
            Source::Exit(exit) => topology.exit(exit).provides.iter().any(|&producer| {
                block
                    .inputs
                    .iter()
                    .any(|input| *flow.wire(producer) == input.ident)
            }),
            Source::Junction(_) => false,
        };
        let at = match block.kind {
            BlockKind::Return if block.transition_target.is_none() => end,
            // A for cycle's body has no `continue` of its own, and its hidden
            // one captures nothing.
            BlockKind::Continue => block.parent.and_then(|header| {
                topology
                    .cycles
                    .iter()
                    .find(|cycle| cycle.header == header)
                    .map(|cycle| Vertex::Junction(cycle.tail))
            }),
            BlockKind::Export => topology
                .cycle_boundaries
                .iter()
                .find(|boundary| Some(boundary.header) == block.export_target)
                .and_then(|boundary| boundary.results.get(flow.exported_output(index)))
                .filter(|result| !provides(result))
                .map(|&result| Vertex::from(result)),
            _ => None,
        };
        if let Some(at) = at {
            captured
                .entry(at)
                .or_default()
                .extend(block.inputs.iter().map(|input| &input.ident));
        }
    }
    let mut on_entry = Captures::new();
    for boundary in &topology.cycle_boundaries {
        let inputs = &flow.blocks[boundary.header].inputs;
        on_entry.entry(boundary.entry).or_default().extend(
            inputs
                .iter()
                .filter(|input| !input.derived)
                .map(|input| &input.ident),
        );
    }
    (captured, on_entry)
}

/// The edges in an order where each connection follows those entering the
/// vertex it leaves. Back edges, which close cycles, come last.
fn forward(before: &[Vec<usize>], after: &[Vec<usize>], connections: usize) -> Vec<usize> {
    let mut waiting: Vec<usize> = before
        .iter()
        .map(|earlier| earlier.iter().filter(|&&edge| edge < connections).count())
        .collect();
    let mut ready: Vec<usize> = (0..connections)
        .filter(|&edge| waiting[edge] == 0)
        .collect();
    let mut order = Vec::with_capacity(before.len());
    while let Some(edge) = ready.pop() {
        order.push(edge);
        for &later in after[edge].iter().filter(|&&later| later < connections) {
            waiting[later] -= 1;
            if waiting[later] == 0 {
                ready.push(later);
            }
        }
    }
    // Connections alone close no cycle; should they ever, the edges left
    // waiting still settle, if slower.
    order.extend((0..before.len()).filter(|&edge| edge >= connections || waiting[edge] > 0));
    order
}

/// Recomputes each set from its neighbours, visiting edges in `order`, until
/// none changes. The sets only grow, so a cycle's back edge settles in a few
/// rounds.
fn settle<T: PartialEq>(sets: &mut [T], order: &[usize], next: impl Fn(usize, &[T]) -> T) {
    let mut changed = true;
    while changed {
        changed = false;
        for &index in order {
            let set = next(index, sets);
            if set != sets[index] {
                sets[index] = set;
                changed = true;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use kaalang_testing::corpus::{self, Suite};
    use proc_macro2::Ident;

    use super::Ribbons;
    use crate::{
        Flow, ProducerId, SemanticModel, SourceFile, Target,
        tests::{fixture, model},
        topology::{Connection, NodeId, NodeKind, Vertex},
    };

    /// The displayed names of `wires`.
    fn names(flow: &Flow, wires: &[ProducerId]) -> Vec<String> {
        wires
            .iter()
            .map(|&producer| flow.wire_name(flow.wire(producer)))
            .collect()
    }

    /// Each connection's ribbon as `from -> to: wires`, a node named by its
    /// description or its kind, and a wire by its displayed name.
    fn ribbons(model: &SemanticModel) -> Vec<String> {
        lines(model, &model.ribbons())
    }

    /// [`ribbons`], of ribbons already found: connections, then back edges.
    fn lines(model: &SemanticModel, ribbons: &Ribbons) -> Vec<String> {
        let flow = &model.analysis.flow;
        let name = |vertex: Vertex| match vertex {
            Vertex::Junction(_) => "junction".to_owned(),
            Vertex::Node(node) => match node {
                NodeId::Block(block) => flow.blocks[block].description.clone(),
                _ => None,
            }
            .unwrap_or_else(|| format!("{:?}", model.topology.node(node).kind)),
        };
        let line = |edge: &Connection, wires: &Vec<_>| {
            let from = Vertex::from(edge.source);
            let wires = names(flow, wires);
            format!(
                "{} -> {}: {}",
                name(from),
                name(edge.destination),
                wires.join(", ")
            )
        };
        model
            .topology
            .connections
            .iter()
            .zip(&ribbons.connections)
            .chain(model.topology.back_edges.iter().zip(&ribbons.back_edges))
            .map(|(edge, wires)| line(edge, wires))
            .collect()
    }

    fn assert_carries(model: &SemanticModel, expected: &[&str]) {
        let ribbons = ribbons(model);
        for line in expected {
            assert!(
                ribbons.contains(&(*line).to_owned()),
                "{line}\n{ribbons:#?}"
            );
        }
    }

    const COLLECT_STEPS: &str = include_str!("../../../kaalang/tests/flow/cycle/collect_steps.rs");

    #[test]
    fn a_wire_rides_from_its_producer_to_its_last_capture() {
        assert_carries(
            &model(
                r#"#[kaa]
                fn chain(first: u8) -> u8 {
                    #[kaa::action("Double it.")]
                    let second = |first| first * 2;

                    #[kaa::action("Add them.")]
                    let third = |first, second| first + second;

                    |third| return third;
                }"#,
            ),
            &[
                "Start -> Double it.: first",
                "Double it. -> Add them.: first, second",
                "Add them. -> End: third",
            ],
        );
    }

    #[test]
    fn a_borrow_keeps_the_wire_for_a_later_capture() {
        assert_carries(
            &model(
                r#"#[kaa]
                fn keep(text: String) -> usize {
                    #[kaa::action("Measure it.")]
                    let length = |&text| text.len();

                    #[kaa::action("Count it again.")]
                    let total = |text, length| text.len() + length;

                    |total| return total;
                }"#,
            ),
            &[
                "Start -> Measure it.: text",
                "Measure it. -> Count it again.: text, length",
            ],
        );
    }

    #[test]
    fn a_wire_no_block_captures_rides_nowhere() {
        assert_carries(
            &model(
                r#"#[kaa]
                fn spare(first: u8) -> u8 {
                    #[kaa::action("Make a spare.")]
                    let (_spare, second) = |first| (first, first);

                    #[kaa::action("Use the second.")]
                    let third = |second| second;

                    |third| return third;
                }"#,
            ),
            &["Make a spare. -> Use the second.: second"],
        );
    }

    #[test]
    fn a_wire_from_before_a_split_reaches_each_branch_that_captures_it() {
        assert_carries(
            &model(
                r#"#[kaa]
                fn pick(condition: bool, first: u8) -> u8 {
                    #[kaa::question("Take the yes branch?")]
                    let (no, yes) = |condition| condition;

                    #[kaa::action("Keep it.")]
                    let value = |no, first| first;

                    #[kaa::action("Raise it.")]
                    let value = |yes, first| first + 1;

                    |value| return value;
                }"#,
            ),
            &[
                "Take the yes branch? -> Keep it.: first, no",
                "Take the yes branch? -> Raise it.: first, yes",
                "Keep it. -> junction: value",
                "Raise it. -> junction: value",
                "junction -> End: value",
            ],
        );
    }

    #[test]
    fn a_wire_local_to_a_branch_stops_at_its_last_capture() {
        let model = model(
            r#"#[kaa]
            fn local(condition: bool, first: u8) -> u8 {
                #[kaa::question("Take the yes branch?")]
                let (no, yes) = |condition| condition;

                #[kaa::action("Make a local.")]
                let local = |no, first| first;

                #[kaa::action("Use the local.")]
                let value = |local| local;

                #[kaa::action("Raise it.")]
                let value = |yes, first| first + 1;

                |value| return value;
            }"#,
        );
        assert_carries(&model, &["Make a local. -> Use the local.: local"]);
        let ribbons = ribbons(&model);
        let after = ribbons
            .iter()
            .filter(|line| line.starts_with("junction ->"));
        assert!(after.clone().count() > 0, "{ribbons:#?}");
        assert!(
            after.clone().all(|line| !line.contains("local")),
            "{ribbons:#?}"
        );
    }

    #[test]
    fn a_back_edge_carries_the_outer_wires_the_next_iteration_captures() {
        let model =
            crate::build(&fixture(COLLECT_STEPS, "collect_steps")).expect("the fixture is valid");
        let flow = &model.analysis.flow;
        let back_edges: Vec<Vec<String>> = model
            .ribbons()
            .back_edges
            .iter()
            .map(|wires| names(flow, wires))
            .collect();
        assert_eq!(back_edges.len(), 2, "{back_edges:?}");
        for wires in &back_edges {
            assert!(
                wires.contains(&"initial_log".to_owned()) && wires.contains(&"limit".to_owned()),
                "{back_edges:?}"
            );
            assert!(!wires.contains(&"step".to_owned()), "{back_edges:?}");
        }
    }

    #[test]
    fn a_cycle_output_rides_from_where_the_body_completes_it_to_its_capture() {
        assert_carries(
            &model(
                r#"#[kaa]
                fn find(limit: u8) -> Option<u8> {
                    #[kaa::cycle("Look for it.")]
                    let found = loop {
                        #[kaa::question("Any left?")]
                        #[kaa::yes("YES")]
                        #[kaa::no("NO")]
                        let (more, none) = |limit| limit > 0;

                        #[kaa::action("Report none.")]
                        let found = |none| None;

                        #[kaa::question("Look further?")]
                        #[kaa::yes("YES")]
                        #[kaa::no("NO")]
                        let (miss, hit) = |more, limit| limit != 3;

                        #[kaa::action("Report it.")]
                        let found = |hit, limit| Some(limit);

                        |miss| continue;
                    };

                    |found| return found;
                }"#,
            ),
            &[
                "Report none. -> junction: found",
                "Report it. -> junction: found",
                "junction -> End: found",
            ],
        );
    }

    #[test]
    fn a_collapsed_cycle_takes_the_wires_its_body_captures() {
        let model = crate::build_with_options(&fixture(COLLECT_STEPS, "collect_steps"), true)
            .expect("the fixture is valid");
        assert_carries(
            &model,
            &["Start an empty log. -> Collect the first steps.: limit, initial_log"],
        );
    }

    #[test]
    fn a_stage_part_carries_the_wires_it_shares_with_preparation() {
        let model = crate::build(&fixture(
            include_str!("../../../kaalang/tests/flow/stage/count_to.rs"),
            "count_to",
        ))
        .expect("the fixture is valid");
        assert_carries(&model.stages[0], &["StageEntry -> Done?: counter, limit"]);
    }

    #[test]
    fn a_for_cycle_takes_its_header_captures_once_on_entry() {
        assert_carries(
            &model(
                r#"#[kaa]
                fn sum(values: &[i64]) -> i64 {
                    #[kaa::action("Start with a total of zero.")]
                    let mut total = || 0;

                    #[kaa::cycle("Add every value to the total.")]
                    let added = |values| for value in values {
                        #[kaa::action("Add the value.")]
                        |value, &mut total| *total += *value;
                    };

                    |added, total| return total;
                }"#,
            ),
            &[
                "Start -> Start with a total of zero.: values",
                "Start with a total of zero. -> Add every value to the total.: values, total",
                "Add every value to the total. -> Add the value.: total, value",
            ],
        );
    }

    #[test]
    fn a_for_body_carries_the_outer_wires_the_next_item_captures() {
        assert_carries(
            &model(
                r#"#[kaa]
                fn scaled(values: Vec<i64>, scale: i64) -> i64 {
                    #[kaa::action("Start with a total of zero.")]
                    let mut total = || 0;

                    #[kaa::cycle("Add every value.")]
                    let added = |values| for value in values {
                        #[kaa::action("Scale it.")]
                        let scaled = |value, &scale| value * *scale;

                        #[kaa::action("Add it.")]
                        |scaled, &mut total| *total += scaled;
                    };

                    |added, total| return total;
                }"#,
            ),
            &[
                "Scale it. -> Add it.: scale, total, scaled",
                "Add it. -> ForEnd: scale, total",
            ],
        );
    }

    #[test]
    fn a_merge_carries_what_is_captured_where_it_stands() {
        let model = model(
            r#"#[kaa]
            fn walk(limit: u8) {
                #[kaa::cycle("Walk.")]
                let done = loop {
                    #[kaa::question("Done?")]
                    let (more, done) = |&limit| *limit == 0;

                    #[kaa::question("Take a big step?")]
                    let (small, big) = |more| true;

                    #[kaa::action("Step a little.")]
                    let stepped = |small| ();

                    #[kaa::action("Step a lot.")]
                    let stepped = |big| ();

                    |stepped| continue;
                };

                |done| return;
            }"#,
        );
        let flow = &model.analysis.flow;
        let merges: Vec<Vec<String>> = model
            .topology
            .junctions
            .iter()
            .zip(model.ribbons().junctions)
            .filter(|(junction, _)| !junction.merges.is_empty())
            .map(|(_, wires)| names(flow, &wires))
            .collect();
        assert!(
            merges
                .iter()
                .any(|wires| wires.contains(&"stepped".to_owned())),
            "{merges:?}"
        );
    }

    #[test]
    fn a_loop_takes_its_gate_once_on_entry() {
        let model = model(
            r#"#[kaa]
            fn gated(go: bool, limit: u8) -> u8 {
                #[kaa::cycle("Count down.")]
                let done = |go| loop {
                    #[kaa::question("Done?")]
                    let (more, done) = |&limit| *limit == 0;

                    #[kaa::action("Wait.")]
                    let waited = |more| ();

                    |waited| continue;
                };

                |done, limit| return limit;
            }"#,
        );
        let gated: Vec<String> = ribbons(&model)
            .into_iter()
            .filter(|line| {
                let (_, wires) = line.rsplit_once(": ").expect("a line lists its wires");
                wires.split(", ").any(|wire| wire == "go")
            })
            .collect();
        assert!(
            gated.len() == 1 && gated[0].starts_with("Start -> "),
            "{gated:?}"
        );
    }

    /// Calls `check` with every local flow the ordinary fixtures draw, in each
    /// view, and its diagram's name.
    fn each_fixture_model(check: impl Fn(&str, &SemanticModel)) {
        for diagram in corpus::diagrams(Suite::Ordinary) {
            let what = diagram.to_string();
            let file =
                SourceFile::parse(&diagram.source, Target::EMPTY).expect("the fixture parses");
            let flow = file
                .flow(diagram.position, diagram.collapse_cycles)
                .expect("the fixture is valid");
            for model in std::iter::once(&flow.model).chain(&flow.model.stages) {
                check(&what, model);
            }
        }
    }

    /// Whether `producer` is provided inside the body of the cycle at `header`.
    fn local_to(flow: &Flow, producer: ProducerId, header: usize) -> bool {
        match producer {
            ProducerId::BlockOutput { block, .. } => {
                flow.enclosing(block).any(|cycle| cycle == header)
            }
            ProducerId::FlowInput(_) => false,
        }
    }

    #[test]
    fn no_fixture_edge_carries_two_wires_of_one_name() {
        each_fixture_model(|what, model| {
            let ribbons = model.ribbons();
            let edges = ribbons.connections.iter().chain(&ribbons.back_edges);
            for (wires, line) in edges.zip(lines(model, &ribbons)) {
                let unique: BTreeSet<String> =
                    names(&model.analysis.flow, wires).into_iter().collect();
                assert_eq!(unique.len(), wires.len(), "{what}: {line}");
            }
        });
    }

    /// No wire local to a cycle body rides that cycle's back edge, or leaves
    /// the body other than to end or a stage transition.
    #[test]
    fn every_fixture_wire_rides_only_inside_its_scope() {
        each_fixture_model(|what, model| {
            let flow = &model.analysis.flow;
            let topology = &model.topology;
            let ribbons = model.ribbons();
            let edges = topology
                .connections
                .iter()
                .map(|edge| (edge, false))
                .chain(topology.back_edges.iter().map(|edge| (edge, true)));
            let wires = ribbons.connections.iter().chain(&ribbons.back_edges);
            for (((edge, back), wires), line) in edges.zip(wires).zip(lines(model, &ribbons)) {
                let ends = matches!(edge.destination, Vertex::Node(node)
                    if matches!(topology.node(node).kind, NodeKind::End | NodeKind::StageTransition));
                for boundary in &topology.cycle_boundaries {
                    let inside = topology
                        .body_vertices(flow, boundary.header)
                        .contains(&edge.destination)
                        || boundary
                            .results
                            .iter()
                            .any(|&result| Vertex::from(result) == edge.destination);
                    let repeats = back
                        && topology.cycles.iter().any(|cycle| {
                            cycle.header == boundary.header
                                && edge.destination == Vertex::Junction(cycle.entry)
                        });
                    if repeats || !(inside || ends) {
                        assert!(
                            wires.iter().all(|&producer| !local_to(
                                flow,
                                producer,
                                boundary.header
                            )),
                            "{what}: {line}"
                        );
                    }
                }
            }
        });
    }

    #[test]
    fn every_fixture_capture_rides_each_connection_into_it() {
        each_fixture_model(|what, model| {
            let flow = &model.analysis.flow;
            let topology = &model.topology;
            let (at, on_entry) = super::captured(model);
            // What no exit or cycle completion provides rides nowhere.
            let completed = topology.cycle_boundaries.iter().flat_map(|boundary| {
                (0..boundary.results.len()).map(|output| ProducerId::BlockOutput {
                    block: boundary.header,
                    output,
                })
            });
            let provided: BTreeSet<&Ident> = topology
                .exits
                .iter()
                .flat_map(|exit| exit.provides.iter().copied())
                .chain(completed)
                .map(|producer| flow.wire(producer))
                .collect();
            let ribbons = model.ribbons();
            let edges = topology.connections.iter().zip(&ribbons.connections);
            for ((edge, wires), line) in edges.zip(lines(model, &ribbons)) {
                let wires: BTreeSet<&Ident> =
                    wires.iter().map(|&producer| flow.wire(producer)).collect();
                let captured = at.get(&edge.destination).into_iter();
                for wire in captured.chain(on_entry.get(&edge.destination)).flatten() {
                    assert!(
                        !provided.contains(wire) || wires.contains(wire),
                        "{what}: {line} lacks {}",
                        flow.wire_name(wire)
                    );
                }
            }
        });
    }
}
