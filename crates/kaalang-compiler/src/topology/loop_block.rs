//! Loop statements use structural junctions, not computational nodes.

use std::collections::{BTreeMap, BTreeSet};

use super::{
    Analyzed, Connection, Destination, Exit, ExitId, Node, NodeId, NodeKind, Source, Topology,
    Vertex, block_node, destination, represented, sequential_exit,
};
use crate::model::Block;

pub(super) fn project_collapsed(
    index: usize,
    block: &Block,
    completes: bool,
    nodes: &mut Vec<Node>,
    exits: &mut Vec<Exit>,
) {
    nodes.push(block_node(index, NodeKind::Loop));
    if completes {
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
    for execution in model.executions {
        for (position, &block) in execution.blocks.iter().enumerate() {
            let Some(target) = model.flow.blocks[block].break_target else {
                continue;
            };
            let Some(&next) = execution.blocks[position + 1..]
                .iter()
                .find(|&&next| represented(model, structural, next))
            else {
                continue;
            };
            let end = model.flow.blocks[target]
                .loop_end
                .expect("a loop owns a body");
            for loop_ in topology
                .loops
                .iter()
                .filter(|loop_| (target..end).contains(&loop_.header))
            {
                topology.order.push(Connection {
                    source: Source::Junction(loop_.tail),
                    destination: destination(structural, next),
                });
            }
        }
    }
}

/// Initially places each completing cycle's continuation below its body.
/// Compaction may lift it beside the body if boundary checks pass (RFC 0002 §8).
/// Diverging cycles have no result to order.
pub(super) fn order_boundaries(topology: &mut Topology) {
    let mut order = Vec::new();
    for boundary in &topology.loop_boundaries {
        let owns = |block| (boundary.header + 1..boundary.end).contains(&block);
        let Some(result) = boundary.result_junction() else {
            continue;
        };
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
                .loops
                .iter()
                .filter(|loop_| (boundary.header..boundary.end).contains(&loop_.header))
                .map(|loop_| Connection {
                    source: Source::Junction(loop_.tail),
                    destination: Destination::Junction(result),
                }),
        );
        order.extend(
            topology
                .loop_boundaries
                .iter()
                .filter(|nested| owns(nested.header))
                .flat_map(|nested| [Some(nested.entry_junction()), nested.result_junction()])
                .flatten()
                .map(|junction| Connection {
                    source: Source::Junction(junction),
                    destination: Destination::Junction(result),
                }),
        );
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
    let mut replacements = topology
        .loop_boundaries
        .iter()
        .filter_map(|boundary| {
            let result = boundary.result_junction()?;
            let mut incoming = topology.incoming(Destination::Junction(result));
            let source = incoming.next()?.source;
            if incoming.next().is_some() {
                return None;
            }
            if matches!(source, Source::Junction(junction)
                if topology.junctions[junction].merges.is_empty())
            {
                return None;
            }
            (topology
                .outgoing(source.into())
                .filter(|edge| edge.source == source)
                .count()
                == 1)
                .then_some((result, source))
        })
        .collect::<BTreeMap<_, _>>();
    for (&result, &merge) in &replacements {
        if let Source::Junction(merge) = merge {
            topology.junctions[merge].is_break = topology.junctions[result].is_break;
            topology.junctions[merge].is_loop_result = true;
        }
    }
    for boundary in &topology.loop_boundaries {
        if topology
            .loops
            .iter()
            .any(|loop_| loop_.header == boundary.header)
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
            && !topology.loop_boundaries.iter().any(|nested| {
                nested.header > boundary.header && (nested.header + 1..nested.end).contains(&block)
            })
        {
            replacements.insert(
                boundary.entry_junction(),
                Source::Exit(ExitId::of(NodeId::Block(block))),
            );
        }
    }
    if !replacements.is_empty() {
        replace_junctions(topology, &replacements);
    }
    // A sole side exit permits an early tail; compaction still checks the
    // nested body envelopes before bringing rows or columns together.
    // Other arrivals retain the body's precedence below its frame.
    let side_tails = topology
        .loops
        .iter()
        .map(|loop_| Destination::Junction(loop_.tail))
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

/// Remove the redundant junctions and carry their identities through every
/// connection, placement constraint, and cycle boundary.
fn replace_junctions(topology: &mut Topology, replacements: &BTreeMap<usize, Source>) {
    // Completion still follows the whole body. When its result is an existing
    // exit, carry that precedence to the continuation rather than back into
    // the body node, which may also have a repeating branch.
    let mut order = Vec::new();
    for &edge in &topology.order {
        if matches!(edge.destination, Vertex::Junction(junction)
            if topology.junctions[junction].is_loop_result
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
    for boundary in &mut topology.loop_boundaries {
        boundary.entry = vertex(boundary.entry);
        boundary.result = boundary.result.map(source);
    }
    for loop_ in &mut topology.loops {
        loop_.entry = junction(loop_.entry);
        loop_.tail = junction(loop_.tail);
    }
    topology.vertices = super::vertices(&topology.nodes, topology.junctions.len());
    topology.index();
}

/// Follow the side occupied by the repeating routes of the first selection.
/// Without a common rightmost branch, the back edge starts on the left contour.
pub(super) fn prefer_left(model: &Analyzed<'_>, header: usize) -> bool {
    let end = model.flow.blocks[header]
        .loop_end
        .expect("a loop owns a body");
    let Some(first) = (header + 1..end).find(|&index| model.flow.blocks[index].branch_count() > 0)
    else {
        return true;
    };
    let last = model.flow.blocks[first].branch_count() - 1;
    model
        .executions
        .iter()
        .filter(|execution| execution.repeats.contains(&header))
        .any(|execution| execution.selected(first) != Some(last))
}
