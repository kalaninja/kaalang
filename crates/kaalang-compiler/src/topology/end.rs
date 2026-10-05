//! Projects the implicit end node.

use std::collections::BTreeSet;

use super::{Destination, Node, NodeId, NodeKind, Topology, Vertex, block_node};
use crate::model::{BlockKind, Flow};

/// End has no exits and no description of its own: its caption is the flow's
/// return type, which a presentation derives from the authored source.
pub(super) fn project(index: usize, nodes: &mut Vec<Node>) {
    nodes.push(block_node(index, NodeKind::End));
}

/// The visual destination of the structural return. The return is an execution
/// transfer rather than a vertex, so its dependencies meet at end itself.
pub(super) fn destination(flow: &Flow) -> Destination {
    let index = flow.blocks.len() - 1;
    debug_assert_eq!(flow.blocks[index].kind, BlockKind::End);
    Destination::Node(NodeId::Block(index))
}

/// Orders every other sink before end, thereby covering the whole DAG.
pub(super) fn order(topology: &mut Topology) {
    let Some(end) = topology
        .nodes
        .iter()
        .find(|node| node.kind == NodeKind::End)
    else {
        return;
    };
    let end = Vertex::Node(end.id);
    let predecessors = topology
        .connections
        .iter()
        .chain(&topology.order)
        .map(|edge| Vertex::from(edge.source))
        .collect::<BTreeSet<_>>();
    let sinks = topology
        .vertices
        .iter()
        .copied()
        .filter(|vertex| *vertex != end && !predecessors.contains(vertex));
    super::order_before(&mut topology.order, sinks, [end]);
}
