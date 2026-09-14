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
    if topology
        .vertices
        .iter()
        .any(|vertex| *vertex != end && arrangement.rank[vertex] >= arrangement.rank[&end])
    {
        return Err(
            "end must be below every other node and junction, including iteration back edges"
                .to_owned(),
        );
    }
    Ok(())
}
