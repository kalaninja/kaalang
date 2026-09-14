//! Loop statements use structural junctions, not computational nodes.

use std::collections::BTreeMap;

use super::{
    Analyzed, Connection, Destination, Exit, ExitId, Node, NodeId, NodeKind, Source, Topology,
    block_node, destination, represented,
};
use crate::model::{Block, ProducerId};

pub(super) fn project_collapsed(
    index: usize,
    block: &Block,
    completes: bool,
    nodes: &mut Vec<Node>,
    exits: &mut Vec<Exit>,
) {
    nodes.push(block_node(index, NodeKind::Loop));
    if completes {
        exits.push(Exit {
            id: ExitId::of(NodeId::Block(index)),
            provides: (0..block.outputs.len())
                .map(|output| ProducerId::BlockOutput {
                    block: index,
                    output,
                })
                .collect(),
        });
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
                .find(|&&next| represented(model, structural, next, false))
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

/// Keeps each expanded cycle's result interface below its whole body. A
/// diverging cycle has no forward interface, so placement-only source order
/// keeps its sibling nodes outside the boundary instead. Completing routes
/// already order their real predecessors and continuations.
pub(super) fn order_boundaries(topology: &mut Topology) {
    let mut order = Vec::new();
    for boundary in &topology.loop_boundaries {
        let owns = |block| (boundary.header + 1..boundary.end).contains(&block);
        let Some(result) = boundary.result else {
            let block = |node| match node {
                NodeId::Block(block) | NodeId::Case { choice: block, .. } => Some(block),
                NodeId::Start => None,
            };
            order.extend(topology.nodes.iter().filter_map(|node| {
                let block = block(node.id)?;
                (block < boundary.header).then_some(Connection {
                    source: Source::Exit(ExitId::of(node.id)),
                    destination: Destination::Junction(boundary.entry),
                })
            }));
            if let Some(tail) = topology
                .loops
                .iter()
                .find(|loop_| loop_.header == boundary.header)
                .map(|loop_| loop_.tail)
            {
                order.extend(topology.nodes.iter().filter_map(|node| {
                    let block = block(node.id)?;
                    (block >= boundary.end).then_some(Connection {
                        source: Source::Junction(tail),
                        destination: Destination::Node(node.id),
                    })
                }));
            }
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
                .flat_map(|nested| [Some(nested.entry), nested.result])
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
