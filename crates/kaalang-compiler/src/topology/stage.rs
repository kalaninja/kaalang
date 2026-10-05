//! Keeps stage transitions below every local route, including divergent cycles.

use std::collections::BTreeSet;

use super::{NodeKind, Topology, Vertex};

pub(super) fn order(topology: &mut Topology) {
    let transitions = topology
        .nodes
        .iter()
        .filter(|node| node.kind == NodeKind::StageTransition)
        .map(|node| Vertex::Node(node.id))
        .collect::<BTreeSet<_>>();
    if transitions.is_empty() {
        return;
    }
    let predecessors = topology
        .connections
        .iter()
        .chain(&topology.order)
        .filter(|edge| !transitions.contains(&edge.destination))
        .map(|edge| Vertex::from(edge.source))
        .collect::<BTreeSet<_>>();
    let sinks = topology
        .vertices
        .iter()
        .copied()
        .filter(|vertex| !transitions.contains(vertex) && !predecessors.contains(vertex));
    super::order_before(&mut topology.order, sinks, transitions.iter().copied());
}
