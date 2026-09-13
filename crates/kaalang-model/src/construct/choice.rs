//! A choice's cases occupy one row in authored order (RFC 0002 §4.5).

use std::collections::BTreeMap;

use crate::topology::{NodeId, Topology, Vertex};

use super::Arrangement;

/// Each vertex is one event, except that the first case represents its whole
/// choice's case row. Other cases have no event of their own.
pub(super) fn events(topology: &Topology) -> Vec<Vec<usize>> {
    let mut events = (0..topology.vertices.len())
        .map(|i| vec![i])
        .collect::<Vec<_>>();
    let mut choices = BTreeMap::<usize, Vec<usize>>::new();
    for (i, vertex) in topology.vertices.iter().enumerate() {
        if let Vertex::Node(NodeId::Case { choice, .. }) = vertex {
            choices.entry(*choice).or_default().push(i);
        }
    }
    for cases in choices.into_values() {
        let first = cases[0];
        for &case in &cases[1..] {
            events[case].clear();
        }
        events[first] = cases;
    }
    events
}

pub(super) fn verify(topology: &Topology, arrangement: &Arrangement) -> Result<(), String> {
    let mut rows = BTreeMap::new();
    for node in &topology.nodes {
        if let NodeId::Case { choice, .. } = node.id {
            let rank = arrangement.rank[&Vertex::Node(node.id)];
            if *rows.entry(choice).or_insert(rank) != rank {
                return Err(format!(
                    "choice {} draws its cases on different rows",
                    choice + 1
                ));
            }
        }
    }
    Ok(())
}
