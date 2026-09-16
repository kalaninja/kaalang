//! Projects the implicit end node.

use std::collections::BTreeSet;

use super::{
    Connection, Destination, ExitId, Node, NodeId, NodeKind, Source, Topology, Vertex, block_node,
};
use crate::model::{BlockKind, Flow};

/// End has no exits and no description of its own: its caption is the flow's
/// return type, which a presentation derives from the authored source
/// (RFC 0002 §4.7).
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
/// Placement-only edges use `ExitId::of` even for nodes without that exit:
/// readers inspect only the source vertex, and these edges are never drawn.
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
    topology.order.extend(
        topology
            .vertices
            .iter()
            .copied()
            .filter(|vertex| *vertex != end && !predecessors.contains(vertex))
            .map(|vertex| Connection {
                source: match vertex {
                    Vertex::Node(node) => Source::Exit(ExitId::of(node)),
                    Vertex::Junction(junction) => Source::Junction(junction),
                },
                destination: end,
            }),
    );
    topology.order.sort_unstable();
    topology.order.dedup();
}
