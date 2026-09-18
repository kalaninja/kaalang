//! Checks that end finishes the entire diagram, including iteration back edges.

use crate::topology::{NodeKind, Topology, Vertex};

use super::Arrangement;

/// Read the final-row rule directly, independently of the projected order.
pub(super) fn verify(topology: &Topology, arrangement: &Arrangement) -> Result<(), String> {
    let Some(end) = topology
        .nodes
        .iter()
        .find(|node| node.kind == NodeKind::End)
    else {
        return Ok(());
    };
    let end = Vertex::Node(end.id);
    let Some(&end_rank) = arrangement.rank.get(&end) else {
        return Err(format!("{end:?} has no rank"));
    };
    for &vertex in &topology.vertices {
        let Some(&rank) = arrangement.rank.get(&vertex) else {
            return Err(format!("{vertex:?} has no rank"));
        };
        if vertex != end && rank >= end_rank {
            return Err(
                "end must be below every other node and junction, including iteration back edges"
                    .to_owned(),
            );
        }
    }
    Ok(())
}
