//! Projects loop conditions and separates repetition from forward precedence.

use std::collections::BTreeSet;

use kaalang_model::{Block, SemanticModel};

use super::{
    Connection, Destination, Exit, Junction, Node, NodeId, NodeKind, Source, Topology, Vertex,
    block_node, question,
};

pub(super) fn project(index: usize, block: &Block, nodes: &mut Vec<Node>, exits: &mut Vec<Exit>) {
    nodes.push(block_node(index, block, NodeKind::Question));
    exits.extend(
        block
            .question_branches
            .iter()
            .enumerate()
            .map(|(branch, answer)| Exit {
                id: question::exit(index, branch),
                handover: Vec::new(),
                branch_description: Some(
                    answer
                        .description
                        .clone()
                        .unwrap_or_else(|| if answer.is_yes { "YES" } else { "NO" }.to_owned()),
                ),
            }),
    );
}

pub(crate) struct Loop {
    pub(crate) header: usize,
    pub(crate) entry: usize,
    pub(crate) tail: usize,
}

/// Normalized traces contain one pass followed by exit. Tail-to-continuation
/// edges constrain placement only: execution returns to the condition instead.
pub(super) fn close(topology: &mut Topology, model: &SemanticModel, wire_merges: usize) {
    for (offset, header) in headers(model).into_iter().enumerate() {
        let tail = wire_merges + offset;
        let entry = topology.junctions.len();
        topology.junctions.push(Junction { wires: Vec::new() });
        topology.vertices.push(Vertex::Junction(entry));
        let condition = Destination::Node(NodeId::Block(header));
        let source = Source::Junction(tail);
        let mut forward = Vec::new();
        for mut connection in std::mem::take(&mut topology.connections) {
            if connection.destination == condition {
                connection.destination = Destination::Junction(entry);
            }
            if connection.source == source {
                topology.order.push(connection);
            } else {
                forward.push(connection);
            }
        }
        for connection in &mut topology.order {
            if connection.destination == condition {
                connection.destination = Destination::Junction(entry);
            }
        }
        forward.push(Connection {
            source: Source::Junction(entry),
            destination: condition,
        });
        topology.connections = forward;
        topology.back_edges.push(Connection {
            source,
            destination: Destination::Junction(entry),
        });
        topology.loops.push(Loop {
            header,
            entry,
            tail,
        });
    }
    defer_continuations(topology);
}

/// Keep joins, iteration tails, and end below the preceding body. The blocks
/// leading to them may fill their branch columns immediately after the NO exit.
/// Walk only after every loop has separated its return from forward execution.
fn defer_continuations(topology: &mut Topology) {
    for connection in std::mem::take(&mut topology.order) {
        let mut pending = vec![connection.destination];
        let mut seen = BTreeSet::new();
        while let Some(destination) = pending.pop() {
            if !seen.insert(destination) {
                continue;
            }
            let boundary = match destination {
                Vertex::Node(node) => topology.node(node).kind == NodeKind::End,
                Vertex::Junction(junction) => {
                    !topology.loops.iter().any(|loop_| loop_.entry == junction)
                }
            };
            if boundary {
                topology.order.push(Connection {
                    source: connection.source,
                    destination,
                });
            } else {
                pending.extend(topology.outgoing(destination).map(|edge| edge.destination));
            }
        }
    }
}

/// Terminal-only bodies never reach a tail and need no return connection.
pub(super) fn headers(model: &SemanticModel) -> Vec<usize> {
    model
        .executions
        .iter()
        .flat_map(|execution| execution.repeats.iter().copied())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}
