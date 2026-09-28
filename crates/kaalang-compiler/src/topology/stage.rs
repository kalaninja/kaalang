//! Keeps stage transitions below every local route, including divergent cycles.

use std::collections::BTreeSet;

use super::{Connection, ExitId, NodeKind, Source, Topology, Vertex};

pub(super) fn order(topology: &mut Topology) {
    let transitions = topology
        .nodes
        .iter()
        .filter(|node| node.kind == NodeKind::Transition)
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
    for &vertex in &topology.vertices {
        if transitions.contains(&vertex) || predecessors.contains(&vertex) {
            continue;
        }
        for &destination in &transitions {
            topology.order.push(Connection {
                source: match vertex {
                    Vertex::Node(node) => Source::Exit(ExitId::of(node)),
                    Vertex::Junction(junction) => Source::Junction(junction),
                },
                destination,
            });
        }
    }
    topology.order.sort_unstable();
    topology.order.dedup();
}
