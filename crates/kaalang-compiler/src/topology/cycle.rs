//! Cycle statements use structural junctions, not computational nodes; a
//! for cycle's for-entry and for-end are the exception.

use std::collections::{BTreeMap, BTreeSet};

use super::{
    Analyzed, Connection, Destination, Exit, ExitId, Node, NodeId, NodeKind, Source, Topology,
    Vertex, block_node, branch_exits, destination, represented, sequential_exit, sole,
};
use crate::model::{Block, ExecutionOutcome, Flow, ProducerId};

pub(super) fn project_collapsed(
    index: usize,
    block: &Block,
    completes: bool,
    nodes: &mut Vec<Node>,
    exits: &mut Vec<Exit>,
) {
    nodes.push(block_node(index, NodeKind::Cycle));
    if !completes {
        return;
    }
    // Several outputs leave by branch exits in declaration order, the first
    // down and the others to the right.
    if block.branch_count() > 0 {
        exits.extend(branch_exits(index, block, super::drawn_branch_exit));
    } else {
        exits.push(sequential_exit(index, block));
    }
}

/// Records the exited region's precedence. Projection carries it through the
/// continuation to its merge, iteration tail, or end boundary.
pub(super) fn order_exits(
    model: &Analyzed<'_>,
    structural: &BTreeMap<usize, usize>,
    topology: &mut Topology,
) {
    if let Some(symbolic) = model.symbolic {
        symbolic.clone().order_exits(model, structural, topology);
        return;
    }
    for execution in model.executions {
        for (position, &block) in execution.blocks.iter().enumerate() {
            let Some(target) = model.flow.blocks[block].export_target else {
                continue;
            };
            let Some(&next) = execution.blocks[position + 1..]
                .iter()
                .find(|&&next| represented(model, structural, next))
            else {
                continue;
            };
            let end = model.flow.blocks[target]
                .cycle_end
                .expect("a cycle owns a body");
            topology.order_iteration_ends(target..end, destination(model, structural, next));
        }
    }
}

/// Initially places each completing cycle's continuation below its body.
/// Compaction may lift it beside the body if boundary checks pass.
/// Diverging cycles have no result to order.
pub(super) fn order_boundaries(topology: &mut Topology) {
    let mut order = Vec::new();
    for boundary in &topology.cycle_boundaries {
        let owns = |block| (boundary.header + 1..boundary.end).contains(&block);
        // A completing loop cycle continues from its result junctions, a for
        // cycle from its for-end.
        let targets = match boundary.caps {
            Some(caps) => vec![Destination::Node(caps.bottom)],
            None => boundary
                .result_junctions()
                .map(Destination::Junction)
                .collect(),
        };
        for &target in &targets {
            order.extend(topology.nodes.iter().filter_map(|node| {
                let block = match node.id {
                    NodeId::Block(block) | NodeId::Case { choice: block, .. } => block,
                    NodeId::Start => return None,
                };
                (owns(block) && Vertex::Node(node.id) != target).then_some(Connection {
                    source: Source::Exit(ExitId::of(node.id)),
                    destination: target,
                })
            }));
            order.extend(
                topology
                    .cycles
                    .iter()
                    .filter(|cycle| (boundary.header..boundary.end).contains(&cycle.header))
                    .map(|cycle| Connection {
                        source: Source::Junction(cycle.tail),
                        destination: target,
                    }),
            );
            // A nested for cycle's caps are body nodes, ordered above.
            order.extend(
                topology
                    .cycle_boundaries
                    .iter()
                    .filter(|nested| owns(nested.header))
                    .flat_map(|nested| {
                        std::iter::once(nested.entry)
                            .chain(nested.results.iter().map(|&result| Vertex::from(result)))
                    })
                    .filter_map(|vertex| match vertex {
                        Vertex::Junction(junction) => Some(Connection {
                            source: Source::Junction(junction),
                            destination: target,
                        }),
                        Vertex::Node(_) => None,
                    }),
            );
        }
    }
    order.retain(|edge| !topology.connections.contains(edge));
    topology.order.extend(order);
    topology.order.sort_unstable();
    topology.order.dedup();
}

/// Reuse body nodes or a final merge when no separate cycle interface is needed.
/// A sole side exit may reach an enclosing tail beside the completed body.
/// An absent back edge or convergence must not reserve an empty row.
pub(super) fn coalesce_boundaries(topology: &mut Topology) {
    // A junction whose sole arrival is a merge, or an exit where `exits` allows
    // one, may become that source when the source feeds nothing else.
    let view: &Topology = topology;
    let reuse = |junction, exits: bool| {
        let source = sole(view.incoming(Destination::Junction(junction)))?.source;
        let reusable = match source {
            Source::Junction(merge) => !view.junctions[merge].merges.is_empty(),
            Source::Exit(_) => exits,
        };
        (reusable
            && view
                .outgoing(source.into())
                .filter(|edge| edge.source == source)
                .count()
                == 1)
            .then_some((junction, source))
    };
    // The repeating routes of a cycle merge before its one continue; when that
    // merge feeds nothing else, it is the iteration tail. A for cycle's rail
    // likewise, and a sole ending enters the for-end directly.
    let tails = view
        .cycles
        .iter()
        .filter_map(|cycle| reuse(cycle.tail, false))
        .chain(view.cycle_boundaries.iter().filter_map(|boundary| {
            let caps = boundary.caps?;
            let rail = caps.rail?;
            reuse(rail, false).or_else(|| {
                sole(view.incoming(Destination::Junction(rail)))
                    .map(|_| (rail, Source::Exit(ExitId::of(caps.bottom))))
            })
        }))
        .collect::<Vec<_>>();
    // Several results keep their own junctions, which construction orders by
    // output; one may still reuse the merge feeding it.
    let mut replacements = view
        .cycle_boundaries
        .iter()
        .filter(|boundary| boundary.caps.is_none())
        .flat_map(|boundary| {
            let exits = boundary.results.len() < 2;
            boundary
                .result_junctions()
                .filter_map(move |result| reuse(result, exits))
        })
        .collect::<BTreeMap<_, _>>();
    for &merge in replacements.values() {
        if let Source::Junction(merge) = merge {
            topology.junctions[merge].is_cycle_result = true;
        }
    }
    for boundary in &topology.cycle_boundaries {
        if boundary.caps.is_some()
            || topology
                .cycles
                .iter()
                .any(|cycle| cycle.header == boundary.header)
        {
            continue;
        }
        let Some(Vertex::Node(NodeId::Block(block))) =
            sole(topology.outgoing(boundary.entry)).map(|edge| edge.destination)
        else {
            continue;
        };
        if topology.single_arrival(NodeId::Block(block))
            && (boundary.header + 1..boundary.end).contains(&block)
            && !topology.cycle_boundaries.iter().any(|nested| {
                nested.header > boundary.header && (nested.header + 1..nested.end).contains(&block)
            })
        {
            replacements.insert(
                boundary.entry_junction(),
                Source::Exit(ExitId::of(NodeId::Block(block))),
            );
        }
    }
    replacements.extend(tails);
    if !replacements.is_empty() {
        replace_junctions(topology, &replacements);
    }
    // A sole side exit permits an early tail; compaction still checks the
    // nested body envelopes before bringing rows or columns together.
    // Other arrivals retain the body's precedence below its frame.
    let side_tails = topology
        .cycles
        .iter()
        .map(|cycle| Destination::Junction(cycle.tail))
        .filter(|&tail| {
            topology
                .incoming(tail)
                .any(|edge| topology.same_row_junction(edge))
        })
        .collect::<BTreeSet<_>>();
    topology
        .order
        .retain(|edge| !side_tails.contains(&edge.destination));
}

/// Draws a for cycle's hidden choice as its for-entry, which hands over the
/// item. The completion leaves the for-end instead.
pub(super) fn project_for_entry(index: usize, nodes: &mut Vec<Node>, exits: &mut Vec<Exit>) {
    nodes.push(block_node(index, NodeKind::ForEntry));
    exits.push(Exit {
        id: ExitId::of(NodeId::Block(index)),
        provides: vec![ProducerId::BlockOutput {
            block: index,
            output: 0,
        }],
    });
}

/// Draws a for cycle's hidden continue as its for-end, where every iteration
/// ends and from which the completed cycle continues with its declared output.
pub(super) fn project_for_end(
    flow: &Flow,
    index: usize,
    nodes: &mut Vec<Node>,
    exits: &mut Vec<Exit>,
) {
    nodes.push(block_node(index, NodeKind::ForEnd));
    let header = flow.blocks[index]
        .parent
        .expect("the hidden continue ends a for body");
    exits.push(Exit {
        id: ExitId::of(NodeId::Block(index)),
        provides: vec![ProducerId::BlockOutput {
            block: header,
            output: 0,
        }],
    });
}

/// Orders everything in each for body before its for-end.
pub(super) fn precede_for_ends(flow: &Flow, topology: &mut Topology) {
    for position in 0..topology.cycle_boundaries.len() {
        let boundary = &topology.cycle_boundaries[position];
        let Some(caps) = boundary.caps else {
            continue;
        };
        precede_bottom_cap(flow, topology, boundary.header, Vertex::Node(caps.bottom));
    }
}

/// Whether an expanded for cycle begins at `block`. Its header draws nothing
/// of its own: the route arriving there continues at its for-entry.
pub(crate) fn opens_for_cycle(model: &Analyzed<'_>, block: usize) -> bool {
    !model.collapse_cycles && model.flow.for_caps(block).is_some()
}

/// The exit of a for cycle's hidden choice: the item leaves the for-entry, the
/// completion the for-end.
pub(super) fn cap_exit(flow: &Flow, block: usize, output: usize) -> ExitId {
    let header = flow.blocks[block]
        .parent
        .expect("the hidden choice opens a for body");
    let (top, bottom) = flow.for_caps(header).expect("a for cycle draws caps");
    ExitId::of(NodeId::Block(if output == 0 { top } else { bottom }))
}

/// The hop a for cycle's exhausted route makes from its for-entry to its
/// for-end. Nothing is drawn for it, but reduction needs it: a capture that
/// skips the cycle is redundant with the route through it.
pub(crate) fn exhausted_hop(model: &Analyzed<'_>, block: usize) -> Option<Connection> {
    if model.collapse_cycles || !model.flow.takes_next_item(block) {
        return None;
    }
    Some(Connection {
        source: super::exit(model, block, 0),
        destination: Vertex::from(super::exit(model, block, 1)),
    })
}

/// Everything in the body precedes the for-end, which the completed cycle
/// continues from: precedence leaving the body passes through it, and a route
/// that never ends its iteration, such as one inside an endless loop cycle,
/// still sits above it.
fn precede_bottom_cap(flow: &Flow, topology: &mut Topology, header: usize, bottom: Vertex) {
    let body = topology.body_vertices(flow, header);
    for edge in &mut topology.order {
        let source = Vertex::from(edge.source);
        if source != bottom && body.contains(&source) && !body.contains(&edge.destination) {
            edge.destination = bottom;
        }
    }
    let departing = topology
        .connections
        .iter()
        .chain(&topology.order)
        .map(|edge| Vertex::from(edge.source))
        .collect::<BTreeSet<_>>();
    let sinks = body
        .into_iter()
        .filter(|&vertex| vertex != bottom && !departing.contains(&vertex));
    super::order_before(&mut topology.order, sinks, [bottom]);
    let connections = &topology.connections;
    topology.order.retain(|edge| !connections.contains(edge));
}

/// Remove the redundant junctions and carry their identities through every
/// connection, placement constraint, and cycle boundary.
fn replace_junctions(topology: &mut Topology, replacements: &BTreeMap<usize, Source>) {
    // Completion still follows the whole body. When its result is an existing
    // exit, carry that precedence to the continuation rather than back into
    // the body node, which may also have a repeating branch.
    let mut order = Vec::new();
    for &edge in &topology.order {
        if matches!(edge.destination, Vertex::Junction(junction)
            if topology.junctions[junction].is_cycle_result
                && matches!(replacements.get(&junction), Some(Source::Exit(_))))
        {
            order.extend(topology.outgoing(edge.destination).map(|next| Connection {
                source: edge.source,
                destination: next.destination,
            }));
        } else {
            order.push(edge);
        }
    }
    topology.order = order;
    let mut ids = vec![Source::Junction(0); topology.junctions.len()];
    let mut next = 0;
    topology.junctions = std::mem::take(&mut topology.junctions)
        .into_iter()
        .enumerate()
        .filter_map(|(index, junction)| {
            if replacements.contains_key(&index) {
                return None;
            }
            ids[index] = Source::Junction(next);
            next += 1;
            Some(junction)
        })
        .collect();
    for (&old, &replacement) in replacements {
        ids[old] = match replacement {
            Source::Junction(junction) => ids[junction],
            exit @ Source::Exit(_) => exit,
        };
    }
    let vertex = |old| match old {
        Vertex::Junction(junction) => ids[junction].into(),
        node @ Vertex::Node(_) => node,
    };
    let source = |old| match old {
        Source::Junction(junction) => ids[junction],
        exit @ Source::Exit(_) => exit,
    };
    for edges in [
        &mut topology.connections,
        &mut topology.order,
        &mut topology.back_edges,
    ] {
        for edge in edges.iter_mut() {
            edge.source = source(edge.source);
            edge.destination = vertex(edge.destination);
        }
        edges.retain(|edge| Destination::from(edge.source) != edge.destination);
        edges.sort_unstable();
        edges.dedup();
    }
    topology
        .order
        .retain(|edge| !topology.connections.contains(edge));
    let junction = |old| match ids[old] {
        Source::Junction(junction) => junction,
        Source::Exit(_) => unreachable!("repeating entries and tails remain junctions"),
    };
    for boundary in &mut topology.cycle_boundaries {
        boundary.entry = vertex(boundary.entry);
        for result in &mut boundary.results {
            *result = source(*result);
        }
        if let Some(caps) = &mut boundary.caps {
            caps.rail = caps.rail.and_then(|rail| match ids[rail] {
                Source::Junction(junction) => Some(junction),
                Source::Exit(_) => None,
            });
        }
    }
    for cycle in &mut topology.cycles {
        cycle.entry = junction(cycle.entry);
        cycle.tail = junction(cycle.tail);
    }
    topology.vertices = super::vertices(&topology.nodes, topology.junctions.len());
    topology.index();
}

/// Follow the side occupied by the repeating routes of the first selection.
/// Without a common rightmost branch, the back edge starts on the left contour.
pub(super) fn prefer_left(model: &Analyzed<'_>, header: usize) -> bool {
    if let Some(symbolic) = model.symbolic {
        return symbolic.clone().prefer_left(model.flow, header);
    }
    let end = model.flow.blocks[header]
        .cycle_end
        .expect("a cycle owns a body");
    let Some(first) = (header + 1..end).find(|&index| model.flow.draws_branches(index)) else {
        return true;
    };
    let last = model.flow.blocks[first].branch_count() - 1;
    model
        .executions
        .iter()
        .filter(|execution| {
            execution.outcome
                == ExecutionOutcome::Repeat {
                    cycle_index: header,
                }
        })
        .any(|execution| execution.selected(first) != Some(last))
}
