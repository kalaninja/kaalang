//! Cycle statements use structural junctions, not computational nodes.

use std::collections::{BTreeMap, BTreeSet};

use super::{
    Analyzed, Connection, Destination, Exit, ExitId, Node, NodeId, NodeKind, Source, Topology,
    Vertex, block_node, branch_exits, destination, represented, sequential_exit,
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
            for cycle in topology
                .cycles
                .iter()
                .filter(|cycle| (target..end).contains(&cycle.header))
            {
                topology.order.push(Connection {
                    source: Source::Junction(cycle.tail),
                    destination: destination(structural, next),
                });
            }
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
        for result in boundary.result_junctions() {
            order.extend(topology.nodes.iter().filter_map(|node| {
                let block = match node.id {
                    NodeId::Block(block) | NodeId::Case { choice: block, .. } => block,
                    NodeId::Start => return None,
                };
                owns(block).then_some(Connection {
                    source: Source::Exit(ExitId::of(node.id)),
                    destination: Destination::Junction(result),
                })
            }));
            order.extend(
                topology
                    .cycles
                    .iter()
                    .filter(|cycle| (boundary.header..boundary.end).contains(&cycle.header))
                    .map(|cycle| Connection {
                        source: Source::Junction(cycle.tail),
                        destination: Destination::Junction(result),
                    }),
            );
            order.extend(
                topology
                    .cycle_boundaries
                    .iter()
                    .filter(|nested| owns(nested.header))
                    .flat_map(|nested| {
                        std::iter::once(nested.entry_junction()).chain(nested.result_junctions())
                    })
                    .map(|junction| Connection {
                        source: Source::Junction(junction),
                        destination: Destination::Junction(result),
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
    // The repeating routes of a cycle merge before its one continue; when that
    // merge feeds nothing else, it is the iteration tail.
    let tails = topology
        .cycles
        .iter()
        .filter_map(|cycle| {
            let mut incoming = topology.incoming(Destination::Junction(cycle.tail));
            let source = incoming.next()?.source;
            if incoming.next().is_some() {
                return None;
            }
            let Source::Junction(merge) = source else {
                return None;
            };
            (!topology.junctions[merge].merges.is_empty()
                && topology
                    .outgoing(source.into())
                    .filter(|edge| edge.source == source)
                    .count()
                    == 1)
                .then_some((cycle.tail, source))
        })
        .collect::<Vec<_>>();
    let view: &Topology = topology;
    let mut replacements = view
        .cycle_boundaries
        .iter()
        .flat_map(|boundary| {
            // Several results keep their own junctions, which construction
            // orders by output; one may still reuse the merge feeding it.
            let several = boundary.results.len() > 1;
            boundary.result_junctions().filter_map(move |result| {
                let mut incoming = view.incoming(Destination::Junction(result));
                let source = incoming.next()?.source;
                if incoming.next().is_some() {
                    return None;
                }
                match source {
                    Source::Junction(junction) if view.junctions[junction].merges.is_empty() => {
                        return None;
                    }
                    Source::Exit(_) if several => return None,
                    _ => {}
                }
                (view
                    .outgoing(source.into())
                    .filter(|edge| edge.source == source)
                    .count()
                    == 1)
                    .then_some((result, source))
            })
        })
        .collect::<BTreeMap<_, _>>();
    for &merge in replacements.values() {
        if let Source::Junction(merge) = merge {
            topology.junctions[merge].is_cycle_result = true;
        }
    }
    for boundary in &topology.cycle_boundaries {
        if topology
            .cycles
            .iter()
            .any(|cycle| cycle.header == boundary.header)
        {
            continue;
        }
        let mut outgoing = topology.outgoing(boundary.entry);
        let Some(Vertex::Node(NodeId::Block(block))) = outgoing.next().map(|edge| edge.destination)
        else {
            continue;
        };
        let node = Vertex::Node(NodeId::Block(block));
        if outgoing.next().is_none()
            && topology.incoming(node).count() == 1
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

/// Draws each for cycle between two caps instead of a boundary with a back
/// edge. Its hidden choice becomes the for-entry, which hands over the item;
/// its iteration tail becomes the for-end, where every iteration ends and
/// from which the completed cycle continues. The boundary stays undrawn, so
/// nothing outside still passes between the caps.
pub(super) fn cap_for_cycles(flow: &Flow, topology: &mut Topology) {
    let mut replacements = BTreeMap::new();
    let mut caps = Vec::new();
    for position in 0..topology.cycle_boundaries.len() {
        let header = topology.cycle_boundaries[position].header;
        if flow.blocks[header].iteration.is_some() {
            let top = NodeId::Block(header + 1);
            let bottom = NodeId::Block(flow.exports(header).start - 1);
            draw_caps(topology, position, top, bottom, &mut replacements);
            topology.cycle_boundaries[position].caps = Some((top, bottom));
            caps.push((position, top, bottom));
        }
    }
    if caps.is_empty() {
        return;
    }
    topology.nodes.sort_by_key(|node| node.id);
    topology.index();
    replace_junctions(topology, &replacements);
    // With no back edge, an entry that only leads to the for-entry is the for-entry.
    let entries = caps
        .iter()
        .filter_map(|&(position, top, _)| {
            let Vertex::Junction(entry) = topology.cycle_boundaries[position].entry else {
                return None;
            };
            let mut outgoing = topology.outgoing(Vertex::Junction(entry));
            let only = outgoing.next()?.destination;
            (only == Vertex::Node(top)
                && outgoing.next().is_none()
                && topology.incoming(Vertex::Node(top)).count() == 1)
                .then_some((entry, Source::Exit(ExitId::of(top))))
        })
        .collect::<BTreeMap<_, _>>();
    if !entries.is_empty() {
        replace_junctions(topology, &entries);
    }
    for (position, _, bottom) in caps {
        let header = topology.cycle_boundaries[position].header;
        precede_bottom_cap(flow, topology, header, Vertex::Node(bottom));
    }
}

/// Turns one for cycle's choice into its for-entry and its iteration tail into
/// its for-end, dropping the back edge. Junctions it retires are recorded in
/// `replacements` for one renumbering of the whole topology.
fn draw_caps(
    topology: &mut Topology,
    position: usize,
    top: NodeId,
    bottom: NodeId,
    replacements: &mut BTreeMap<usize, Source>,
) {
    let NodeId::Block(next) = top else {
        unreachable!("a for-entry is a block node")
    };
    let header = topology.cycle_boundaries[position].header;
    let case = |branch| {
        ExitId::of(NodeId::Case {
            choice: next,
            branch,
        })
    };
    let is_case = |node: NodeId| matches!(node, NodeId::Case { choice, .. } if choice == next);
    for node in &mut topology.nodes {
        if node.id == top {
            node.kind = NodeKind::ForEntry;
        }
    }
    topology.nodes.retain(|node| !is_case(node.id));
    topology.nodes.push(Node {
        id: bottom,
        kind: NodeKind::ForEnd,
    });
    let item = topology
        .exits
        .iter()
        .find(|exit| exit.id == case(0))
        .map(|exit| exit.provides.clone())
        .unwrap_or_default();
    topology.exits.retain(|exit| !is_case(exit.id.node));
    for exit in &mut topology.exits {
        if exit.id == ExitId::of(top) {
            exit.provides.clone_from(&item);
        }
    }
    let bottom = ExitId::of(bottom);
    topology.exits.push(Exit {
        id: bottom,
        provides: vec![ProducerId::BlockOutput {
            block: header,
            output: 0,
        }],
    });
    // The item leaves the for-entry; running out of items leaves the for-end.
    let moved = |source| match source {
        Source::Exit(exit) if exit == case(0) => Source::Exit(ExitId::of(top)),
        Source::Exit(exit) if exit == case(1) => Source::Exit(bottom),
        source => source,
    };
    for edges in [
        &mut topology.connections,
        &mut topology.order,
        &mut topology.back_edges,
    ] {
        edges.retain(|edge| !matches!(edge.destination, Vertex::Node(node) if is_case(node)));
        for edge in edges.iter_mut() {
            edge.source = moved(edge.source);
        }
    }
    if let Some(cycle) = topology
        .cycles
        .iter()
        .position(|cycle| cycle.header == header)
    {
        let tail = topology.cycles.remove(cycle).tail;
        topology
            .back_edges
            .retain(|edge| edge.source != Source::Junction(tail));
        // Several iteration endings still meet on the tail's rail, then enter
        // the for-end together; a single one enters it directly.
        let endings = topology
            .connections
            .iter()
            .filter(|edge| edge.destination == Vertex::Junction(tail))
            .count();
        if endings > 1 {
            topology.connections.push(Connection {
                source: Source::Junction(tail),
                destination: Vertex::Node(bottom.node),
            });
        } else {
            replacements.insert(tail, Source::Exit(bottom));
        }
    }
    for result in &mut topology.cycle_boundaries[position].results {
        match *result {
            Source::Junction(junction) => {
                replacements.insert(junction, Source::Exit(bottom));
            }
            exit @ Source::Exit(_) => *result = moved(exit),
        }
    }
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
        .filter(|&vertex| vertex != bottom && !departing.contains(&vertex))
        .collect::<Vec<_>>();
    topology
        .order
        .extend(sinks.into_iter().map(|vertex| Connection {
            source: match vertex {
                Vertex::Node(node) => Source::Exit(ExitId::of(node)),
                Vertex::Junction(junction) => Source::Junction(junction),
            },
            destination: bottom,
        }));
    let connections = &topology.connections;
    topology.order.retain(|edge| !connections.contains(edge));
    topology.order.sort_unstable();
    topology.order.dedup();
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
