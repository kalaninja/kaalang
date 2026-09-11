//! The regions a flow's branches carve out of its topology.
//!
//! These are facts about the topology alone: which vertices one branch of a
//! question or choice leads to, and which of them its siblings share. Both the
//! placement and the check read them — the placement to reserve columns, the
//! check to hold the columns it is given to RFC 0002 §8. Sharing a fact is not
//! self-reference; sharing the column arithmetic would be, so that stays apart.

use std::collections::{BTreeMap, BTreeSet};

use crate::model::{BlockKind, Flow};
use crate::topology::{ExitId, NodeId, Topology, Vertex};

/// Every question and choice of a flow, in authored order.
pub(super) fn branchers(flow: &Flow) -> Vec<usize> {
    flow.blocks
        .iter()
        .enumerate()
        .filter(|(_, block)| matches!(block.kind, BlockKind::Question | BlockKind::Choice))
        .map(|(block, _)| block)
        .collect()
}

/// Transitive successors of every vertex.
pub(super) fn reachable(topology: &Topology) -> BTreeMap<Vertex, BTreeSet<Vertex>> {
    // ponytail: one breadth-first walk per vertex, each an indexed neighbour
    // lookup; share one closure if a flow ever passes a few hundred blocks.
    topology
        .vertices
        .iter()
        .map(|&start| {
            let mut seen = BTreeSet::new();
            let mut frontier = vec![start];
            while let Some(vertex) = frontier.pop() {
                for connection in topology.outgoing(vertex) {
                    let next = connection.destination;
                    if seen.insert(next) {
                        frontier.push(next);
                    }
                }
            }
            (start, seen)
        })
        .collect()
}

/// What each branch of one brancher leads to, including the branch's own case
/// node. A vertex in two of these sets belongs to a convergence group.
pub(super) fn branch_sets(
    topology: &Topology,
    flow: &Flow,
    reachable: &BTreeMap<Vertex, BTreeSet<Vertex>>,
    block: usize,
) -> Vec<BTreeSet<Vertex>> {
    (0..flow.blocks[block].branch_count())
        .map(|branch| {
            let heads: Vec<Vertex> = match flow.blocks[block].kind {
                BlockKind::Choice => vec![Vertex::Node(NodeId::Case {
                    choice: block,
                    branch,
                })],
                _ => topology
                    .leaving(ExitId {
                        node: NodeId::Block(block),
                        branch: Some(branch),
                    })
                    .map(|connection| connection.destination)
                    .collect(),
            };
            heads
                .into_iter()
                .flat_map(|head| std::iter::once(head).chain(reachable[&head].iter().copied()))
                .collect()
        })
        .collect()
}

/// The vertices one brancher's own branches draw, and so the area RFC 0002 §8
/// asks it to reserve columns for.
///
/// Two kinds of vertex are reachable from a branch without belonging to the
/// brancher: one every branch reaches, which is where they have fully converged
/// and the flow below the brancher resumes; and one a route can also reach
/// without passing the brancher at all, which belongs to an enclosing
/// brancher's shared continuation and may well sit to the brancher's left. What
/// remains — dominated by the brancher, and not shared by all of its branches —
/// is its own, including the shared continuation of a partial merge.
pub(super) fn footprint_vertices(
    topology: &Topology,
    block: usize,
    branches: &[BTreeSet<Vertex>],
) -> BTreeSet<Vertex> {
    let mut owned = BTreeSet::new();
    for set in branches {
        owned.extend(set.iter().copied());
    }
    let converged = branches.iter().skip(1).fold(
        branches.first().cloned().unwrap_or_default(),
        |common, set| common.intersection(set).copied().collect(),
    );
    let bypassing = bypassing(topology, Vertex::Node(NodeId::Block(block)));
    owned
        .difference(&converged)
        .copied()
        .filter(|vertex| !bypassing.contains(vertex))
        .collect()
}

/// Shared computational continuations, paired with each group's first branch.
/// Their incoming exits determine the current column, including any intervening
/// nested selection. Wider, enclosing continuations are excluded.
pub(super) fn continuations(
    topology: &Topology,
    block: usize,
    branches: &[BTreeSet<Vertex>],
) -> BTreeMap<Vertex, usize> {
    let bypassing = bypassing(topology, Vertex::Node(NodeId::Block(block)));
    let mut members = BTreeMap::<Vertex, Vec<usize>>::new();
    for (branch, vertices) in branches.iter().enumerate() {
        for &vertex in vertices {
            members.entry(vertex).or_default().push(branch);
        }
    }
    members
        .into_iter()
        .filter_map(|(vertex, group)| {
            (group.len() >= 2
                && matches!(vertex, Vertex::Node(NodeId::Block(_)))
                && !bypassing.contains(&vertex))
            .then(|| (vertex, group[0]))
        })
        .collect()
}

/// Every vertex a route reaches from the start node without passing `avoided`.
fn bypassing(topology: &Topology, avoided: Vertex) -> BTreeSet<Vertex> {
    let start = Vertex::Node(NodeId::Start);
    let mut seen = BTreeSet::from([start]);
    let mut frontier = vec![start];
    while let Some(vertex) = frontier.pop() {
        for connection in topology.outgoing(vertex) {
            let next = connection.destination;
            if next != avoided && seen.insert(next) {
                frontier.push(next);
            }
        }
    }
    seen
}

/// The column one branch of a brancher starts in: its case node's column for a
/// choice, and the branch column its exit leaves by for a question.
pub(super) fn branch_column(
    arrangement: &super::Arrangement,
    flow: &Flow,
    block: usize,
    branch: usize,
) -> Option<i32> {
    if flow.blocks[block].kind == BlockKind::Choice {
        return arrangement
            .column
            .get(&Vertex::Node(NodeId::Case {
                choice: block,
                branch,
            }))
            .copied();
    }
    let exit = ExitId {
        node: NodeId::Block(block),
        branch: Some(branch),
    };
    Some(
        arrangement
            .column
            .get(&Vertex::Node(NodeId::Block(block)))?
            + arrangement.exit_offset.get(&exit)?,
    )
}
