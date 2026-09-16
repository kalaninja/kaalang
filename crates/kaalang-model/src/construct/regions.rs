//! The regions a flow's branches carve out of its topology.
//!
//! These are facts about the topology alone: which vertices one branch of a
//! question or choice leads to, and which of them its siblings share. The
//! deciding search's numbering, the check, and the test-only reference all
//! read `Regions::outside` and `Regions::reserved` — the two halves of one
//! definition, each leaving out what the other side also reaches — and one
//! definition is what makes the three agree; the preferred placement reserves
//! from `branch_sets` and is held to the check like everything else. Sharing a
//! fact is not self-reference; sharing the column arithmetic would be, so that
//! stays apart, and the reference builds its own arithmetic for the same
//! reason.

use std::collections::{BTreeMap, BTreeSet};

use crate::model::{BlockKind, Flow};
use crate::topology::{ExitId, NodeId, Source, Topology, Vertex};

/// Every question and choice of a flow, in authored order.
pub(super) fn branchers(flow: &Flow, topology: &Topology) -> Vec<usize> {
    flow.blocks
        .iter()
        .enumerate()
        .filter(|(index, block)| {
            matches!(block.kind, BlockKind::Question | BlockKind::Choice)
                && topology
                    .nodes
                    .iter()
                    .any(|node| node.id == NodeId::Block(*index))
        })
        .map(|(block, _)| block)
        .collect()
}

/// Transitive successors of every vertex.
pub(super) fn reachable(topology: &Topology) -> BTreeMap<Vertex, BTreeSet<Vertex>> {
    // ponytail: one walk per vertex, each an indexed neighbour lookup; one
    // shared closure if a flow ever passes a few hundred blocks.
    topology
        .vertices
        .iter()
        .map(|&start| (start, reached(topology, start, None)))
        .collect()
}

/// Every vertex a route reaches from `start` without entering `avoided`. The
/// start itself appears only when a back edge reaches it.
fn reached(topology: &Topology, start: Vertex, avoided: Option<Vertex>) -> BTreeSet<Vertex> {
    walk(topology, start, avoided, false)
}

/// The same, counting placement precedence as reaching.
///
/// A cycle's iteration tail has no outgoing connection — the projection moved
/// its forward edges into `Topology::order` — so a branch that repeats appears
/// to stop there. Only this walk shows that it goes on to whatever the flow
/// draws below the cycle.
fn carried(topology: &Topology, start: Vertex) -> BTreeSet<Vertex> {
    walk(topology, start, None, true)
}

fn walk(
    topology: &Topology,
    start: Vertex,
    avoided: Option<Vertex>,
    precedence: bool,
) -> BTreeSet<Vertex> {
    let mut seen = BTreeSet::new();
    let mut frontier = vec![start];
    while let Some(vertex) = frontier.pop() {
        let carried = topology
            .order
            .iter()
            .filter(move |edge| precedence && Vertex::from(edge.source) == vertex);
        for connection in topology.outgoing(vertex).chain(carried) {
            let next = connection.destination;
            if Some(next) != avoided && seen.insert(next) {
                frontier.push(next);
            }
        }
    }
    seen
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
            branch_heads(topology, flow, block, branch)
                .into_iter()
                .flat_map(|head| std::iter::once(head).chain(reachable[&head].iter().copied()))
                .collect()
        })
        .collect()
}

/// Where one branch of a brancher begins.
fn branch_heads(topology: &Topology, flow: &Flow, block: usize, branch: usize) -> Vec<Vertex> {
    match flow.blocks[block].kind {
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
    }
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
    flow: &Flow,
    topology: &Topology,
    block: usize,
    branches: &[BTreeSet<Vertex>],
) -> BTreeSet<Vertex> {
    let mut owned = BTreeSet::new();
    for set in branches {
        owned.extend(set.iter().copied());
    }
    // What every branch reaches is where they have fully converged. A branch
    // that repeats reaches it through the iteration tail's placement
    // precedence rather than through a connection, so this reads the carrying
    // walk: otherwise the whole flow below the loop looks like the private
    // continuation of the branches that leave it. Otherwise it is
    // `branch_sets` — every head of the branch, and the head itself.
    let converged = (0..flow.blocks[block].branch_count())
        .map(|branch| {
            branch_heads(topology, flow, block, branch)
                .into_iter()
                .flat_map(|head| std::iter::once(head).chain(carried(topology, head)))
                .collect::<BTreeSet<_>>()
        })
        .reduce(|common, set| common.intersection(&set).copied().collect())
        .unwrap_or_default();
    let bypassing = bypassing(topology, Vertex::Node(NodeId::Block(block)));
    owned
        .difference(&converged)
        .copied()
        .filter(|vertex| !bypassing.contains(vertex))
        .collect()
}

/// One convergence group of a brancher: the branches that meet, and
/// everything those branches draw.
pub(super) struct Group {
    /// The branches that reach a common vertex, in authored order.
    pub(super) members: BTreeSet<usize>,
    /// Every vertex they draw, the shared continuation included.
    pub(super) area: BTreeSet<Vertex>,
}

/// The convergence groups of one brancher, read off the topology.
///
/// A vertex two or more branches reach is where they converge; the branches
/// that reach it are the group, and the area it reserves is everything those
/// branches draw (RFC 0002 §8). A vertex *every* branch reaches is where the
/// brancher has fully converged and the flow below it resumes, so it opens no
/// group, and one a route can reach without passing the brancher belongs to an
/// enclosing group.
///
/// Two meeting points with the same members are one group, so this returns one
/// entry per distinct set of branches.
fn groups(branches: &[BTreeSet<Vertex>], own: &BTreeSet<Vertex>) -> Vec<Group> {
    let mut meeting = BTreeMap::<Vertex, BTreeSet<usize>>::new();
    for (branch, vertices) in branches.iter().enumerate() {
        for vertex in vertices.intersection(own) {
            meeting.entry(*vertex).or_default().insert(branch);
        }
    }
    meeting
        .into_values()
        .filter(|members| members.len() >= 2)
        .collect::<BTreeSet<_>>()
        .into_iter()
        .map(|members| Group {
            area: members
                .iter()
                .flat_map(|&branch| branches[branch].intersection(own).copied())
                .collect(),
            members,
        })
        .collect()
}

/// What one selection carves out of the topology: what each branch leads to,
/// which of that it draws, and which branches converge where.
pub(super) struct Regions {
    /// What each branch leads to, in authored order.
    pub(super) branches: Vec<BTreeSet<Vertex>>,
    /// What the selection draws: dominated by it, not shared by every branch.
    pub(super) own: BTreeSet<Vertex>,
    pub(super) groups: Vec<Group>,
}

impl Regions {
    /// What one branch draws that no branch of `group` also reaches.
    ///
    /// A vertex a member also reaches is part of the group's own area, not of
    /// this branch, so the group's reserved columns say nothing about it. That
    /// is the only exception: a continuation this branch shares with its *own*
    /// co-members is still this branch's side of the diagram, and has to keep
    /// clear of the group like the rest of it.
    pub(super) fn outside(&self, group: &Group, branch: usize) -> BTreeSet<Vertex> {
        self.branches[branch]
            .intersection(&self.own)
            .filter(|vertex| {
                !group
                    .members
                    .iter()
                    .any(|&member| self.branches[member].contains(vertex))
            })
            .copied()
            .collect()
    }

    /// What one group reserves against a later sibling: its area, less
    /// whatever that sibling reaches too (RFC 0002 §8).
    ///
    /// The exception is `outside`'s, read from the other side. A vertex the
    /// sibling also reaches is common ground, and a branch cannot be asked to
    /// keep clear of something it draws itself — which is unsatisfiable for
    /// the first branch, whose column RFC 0002 §8 fixes as the brancher's own.
    ///
    /// Only a sibling written after every member is held to this set. The
    /// mirror image — an earlier sibling held left of the area, an enclosed
    /// one held between the members around it — was tried as a normal form and
    /// refused a flow that has a diagram; RFC 0003 §2.2 records it.
    pub(super) fn reserved(&self, group: &Group, branch: usize) -> BTreeSet<Vertex> {
        group
            .area
            .difference(&self.branches[branch])
            .copied()
            .collect()
    }

    /// The siblings written after every member of one group: the branches RFC
    /// 0002 §8 holds to the right of what the group draws.
    pub(super) fn later_siblings(&self, group: &Group) -> std::ops::Range<usize> {
        let after = group
            .members
            .iter()
            .next_back()
            .map_or(self.branches.len(), |&last| last + 1);
        after..self.branches.len()
    }
}

/// Reads one selection's regions once. Every caller needs all of them, and
/// each costs a walk of the topology.
pub(super) fn regions(
    flow: &Flow,
    topology: &Topology,
    reachable: &BTreeMap<Vertex, BTreeSet<Vertex>>,
    block: usize,
) -> Regions {
    let branches = branch_sets(topology, flow, reachable, block);
    let own = footprint_vertices(flow, topology, block, &branches);
    Regions {
        groups: groups(&branches, &own),
        branches,
        own,
    }
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

/// The exits of `block`'s branch `first` that reach the shared continuation
/// `entry`, read back through the junctions before it. A continuation follows
/// the first branch's actual approach, which may have moved through a nested
/// selection since leaving the brancher.
pub(super) fn first_branch_approaches(
    topology: &Topology,
    branches: &[BTreeSet<Vertex>],
    block: usize,
    first: usize,
    entry: Vertex,
) -> BTreeSet<ExitId> {
    let mut pending = vec![entry];
    let mut approaches = BTreeSet::new();
    while let Some(vertex) = pending.pop() {
        for wire in topology.incoming(vertex) {
            match wire.source {
                Source::Junction(junction) => pending.push(Vertex::Junction(junction)),
                Source::Exit(exit)
                    if branches[first].contains(&Vertex::Node(exit.node))
                        || (exit.node == NodeId::Block(block) && exit.branch == Some(first)) =>
                {
                    approaches.insert(exit);
                }
                Source::Exit(_) => {}
            }
        }
    }
    approaches
}

/// Every vertex a route reaches from the start node without passing `avoided`.
fn bypassing(topology: &Topology, avoided: Vertex) -> BTreeSet<Vertex> {
    let start = Vertex::Node(NodeId::Start);
    let mut seen = reached(topology, start, Some(avoided));
    seen.insert(start);
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
