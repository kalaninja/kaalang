//! Assigns a row and a column to every node, and a row to every junction.
//!
//! RFC 0002 §8 runs execution time from top to bottom, so a row comes from
//! connection reachability, including the chosen serial order.
//! Branches run left to right in authored order, and a brancher reserves the
//! whole footprint its branches occupy, so that nested branchers and the
//! convergence groups they share compose without corrupting each other.

use std::collections::{BTreeMap, BTreeSet};

use kaalang_model::{BlockKind, SemanticModel};

use crate::topology::{
    Connection, Destination, ExitId, NodeId, Source, Topology, Vertex, choice, question,
};

pub(super) struct Placement {
    row: BTreeMap<Vertex, usize>,
    column: BTreeMap<Vertex, usize>,
    footprints: Footprints,
    pub(super) rows: usize,
}

impl Placement {
    pub(super) fn row(&self, vertex: Vertex) -> usize {
        self.row[&vertex]
    }

    pub(super) fn column(&self, vertex: Vertex) -> usize {
        self.column[&vertex]
    }

    pub(super) fn exit_column(&self, exit: ExitId) -> usize {
        self.column(Vertex::Node(exit.node)) + self.footprints.offset(exit)
    }
}

/// Places one projected topology, or reports that it has no top-to-bottom
/// order at all.
pub(super) fn place(
    topology: &Topology,
    model: &SemanticModel,
    delays: &BTreeMap<Vertex, usize>,
) -> Result<Placement, String> {
    let row = rows(topology, delays)?;
    let rows_used = row.values().copied().max().unwrap_or(0) + 1;
    let footprints = footprints(topology, model);
    Ok(Placement {
        column: columns(topology, &footprints, &row, rows_used),
        footprints,
        row,
        rows: rows_used,
    })
}

/// Longest-path ranking: a node sits below every predecessor, a junction below
/// every producer branch and every block ordered before it. Start owns row 0;
/// every computational node is reached from it in the chosen serial order.
///
/// Dependency depth is only the earliest row an item may take. Routing asks for
/// a `delay` when two runs cannot share a row gap, and because a row is derived
/// from already-delayed predecessors, the delay carries to everything
/// downstream on its own.
fn rows(
    topology: &Topology,
    delays: &BTreeMap<Vertex, usize>,
) -> Result<BTreeMap<Vertex, usize>, String> {
    let mut rows = BTreeMap::new();
    let mut pending = topology.vertices.iter().copied().collect::<BTreeSet<_>>();
    while !pending.is_empty() {
        // Validated wire merges leave an acyclic visual graph, so this only
        // fails on a projection bug. Refuse the diagram rather than abort: the
        // renderer is also a library and a command-line binary.
        let Some(&ready) = pending.iter().find(|&&vertex| {
            topology
                .incoming(vertex)
                .all(|connection| rows.contains_key(&Vertex::from(connection.source)))
        }) else {
            return Err("the connections form a cycle, so no node can be lowest".to_owned());
        };
        pending.remove(&ready);
        let row = topology
            .incoming(ready)
            .map(|connection| Vertex::from(connection.source))
            .map(|predecessor| rows[&predecessor] + 1)
            .max()
            .unwrap_or(usize::from(ready != Vertex::Node(NodeId::Start)));
        rows.insert(ready, row + delays.get(&ready).copied().unwrap_or(0));
    }
    Ok(rows)
}

/// The columns of one brancher's branches, as offsets from its own column. The
/// first branch continues down the brancher's column and later branches sit to
/// its right, in authored order.
struct Footprints {
    /// Branch offsets per branching block.
    offsets: BTreeMap<usize, Vec<usize>>,
    /// Total columns owned per branching block.
    width: BTreeMap<usize, usize>,
}

impl Footprints {
    fn offset(&self, exit: ExitId) -> usize {
        match (exit.node, exit.branch) {
            (NodeId::Block(block), Some(branch)) => self.offsets[&block][branch],
            _ => 0,
        }
    }

    fn span(&self, node: NodeId) -> usize {
        match node {
            NodeId::Block(block) => self.width.get(&block).copied().unwrap_or(1),
            _ => 1,
        }
    }
}

fn columns(
    topology: &Topology,
    footprints: &Footprints,
    rows: &BTreeMap<Vertex, usize>,
    rows_used: usize,
) -> BTreeMap<Vertex, usize> {
    let mut ordered = topology.vertices.clone();
    ordered.sort_by_key(|vertex| (rows[vertex], *vertex));

    let mut columns = BTreeMap::new();
    let mut occupied = vec![0; rows_used];
    for vertex in ordered {
        let Vertex::Node(node) = vertex else {
            // A junction owns no column of its own; it is placed once the
            // consumers its common segment reaches have theirs.
            continue;
        };
        let row = rows[&vertex];
        let preferred = preferred(topology, &columns, footprints, node);
        let column = preferred.max(occupied[row]);
        occupied[row] = column + footprints.span(node);
        columns.insert(vertex, column);
    }

    let mut claimed = columns
        .iter()
        .map(|(vertex, &column)| (rows[vertex], column))
        .collect::<BTreeSet<_>>();
    // Deepest junction first. Two merges feeding one chain both want its
    // column, and the deeper one is the one already beside what it feeds;
    // letting the shallower one move keeps their producers' runs from having to
    // swap sides, which no lane order and no extra row can undo.
    let mut junctions = (0..topology.junctions.len())
        .map(|junction| (rows[&Vertex::Junction(junction)], junction))
        .collect::<Vec<_>>();
    junctions.sort_unstable_by(|left, right| right.cmp(left));
    for (_, junction) in junctions {
        let vertex = Vertex::Junction(junction);
        // The common segment out of the junction descends into the leftmost
        // consumer. Only items on the same row claim this point: successive
        // merges may reuse the same column.
        let preferred = topology
            .outgoing(vertex)
            .filter_map(|connection| match connection.destination {
                Destination::Node(node) => columns.get(&Vertex::Node(node)).copied(),
                Destination::Junction(_) => None,
            })
            .min()
            .unwrap_or(0);
        let mut column = preferred;
        let row = rows[&vertex];
        while !claimed.insert((row, column)) {
            column += 1;
        }
        columns.insert(vertex, column);
    }

    columns
}

/// A node prefers its leftmost predecessor's column, offset by the branch it
/// arrives on. A case prefers its own branch's offset inside its select, and a
/// root prefers the leftmost column.
fn preferred(
    topology: &Topology,
    columns: &BTreeMap<Vertex, usize>,
    footprints: &Footprints,
    node: NodeId,
) -> usize {
    if let NodeId::Case { choice, branch } = node {
        let select = columns[&Vertex::Node(NodeId::Block(choice))];
        return select + footprints.offsets[&choice][branch];
    }

    // RFC 0002 §8 puts branches in authored order left to right, so a branch's
    // own column decides where its successor goes; only a node no branch
    // reaches falls back to its leftmost predecessor.
    let mut arrivals = topology.incoming(Vertex::Node(node)).peekable();
    if arrivals.peek().is_none() {
        return 0;
    }
    let branches = topology
        .incoming(Vertex::Node(node))
        .filter(|connection| from_branch(connection.source))
        .map(|connection| arrives_from(topology, columns, footprints, connection))
        .min();

    branches.unwrap_or_else(|| {
        arrivals
            .map(|connection| arrives_from(topology, columns, footprints, connection))
            .min()
            .unwrap_or(0)
    })
}

/// Whether a connection leaves a branch of a question or a case of a choice.
const fn from_branch(source: Source) -> bool {
    matches!(
        source,
        Source::Exit(
            ExitId {
                branch: Some(_),
                ..
            } | ExitId {
                node: NodeId::Case { .. },
                ..
            }
        )
    )
}

fn arrives_from(
    topology: &Topology,
    columns: &BTreeMap<Vertex, usize>,
    footprints: &Footprints,
    connection: &Connection,
) -> usize {
    match connection.source {
        Source::Exit(exit) => columns[&Vertex::Node(exit.node)] + footprints.offset(exit),
        // A junction is placed after every node, so a consumer reads the
        // producers that meet in it instead.
        Source::Junction(junction) => topology
            .incoming(Vertex::Junction(junction))
            .map(|producer| arrives_from(topology, columns, footprints, producer))
            .min()
            .unwrap_or(0),
    }
}

fn footprints(topology: &Topology, model: &SemanticModel) -> Footprints {
    let reachable = reachable(topology);
    let mut footprints = Footprints {
        offsets: BTreeMap::new(),
        width: BTreeMap::new(),
    };
    // Deepest first, so a nested brancher's width is known before the brancher
    // whose branch contains it asks for it.
    let mut ordered = branchers(model);
    ordered.sort_by_key(|&block| reachable[&Vertex::Node(NodeId::Block(block))].len());
    for block in ordered {
        let branches = branch_sets(topology, model, &reachable, block);
        let mut members = BTreeMap::<Vertex, Vec<usize>>::new();
        for (branch, set) in branches.iter().enumerate() {
            for &vertex in set {
                members.entry(vertex).or_default().push(branch);
            }
        }
        let mut groups = BTreeMap::<Vec<usize>, BTreeSet<Vertex>>::new();
        for (vertex, members) in members {
            groups.entry(members).or_default().insert(vertex);
        }
        let mut offsets = Vec::with_capacity(branches.len());
        let mut width = 0;
        for branch in 0..branches.len() {
            offsets.push(width);
            width += 1;
            // A shared continuation starts at its first branch. Reserve its
            // whole footprint after the group's last branch, before placing
            // the next case, rather than only widening the enclosing brancher.
            for (members, set) in &groups {
                if members.last() == Some(&branch) {
                    width = width.max(offsets[members[0]] + width_of(set, &reachable, &footprints));
                }
            }
        }
        footprints.width.insert(block, width);
        footprints.offsets.insert(block, offsets);
    }
    footprints
}

fn branchers(model: &SemanticModel) -> Vec<usize> {
    model
        .flow
        .blocks
        .iter()
        .enumerate()
        .filter(|(_, block)| matches!(block.kind, BlockKind::Question | BlockKind::Choice))
        .map(|(block, _)| block)
        .collect()
}

/// What each branch of one brancher leads to, including the branch's own case
/// node. A vertex in two of these sets belongs to a convergence group.
fn branch_sets(
    topology: &Topology,
    model: &SemanticModel,
    reachable: &BTreeMap<Vertex, BTreeSet<Vertex>>,
    block: usize,
) -> Vec<BTreeSet<Vertex>> {
    (0..model.flow.blocks[block].outputs.len())
        .map(|branch| {
            let heads: Vec<Vertex> = match model.flow.blocks[block].kind {
                BlockKind::Choice => vec![Vertex::Node(choice::case(block, branch))],
                _ => topology
                    .leaving(question::exit(block, branch))
                    .map(|connection| Vertex::from(connection.destination))
                    .collect(),
            };
            heads
                .into_iter()
                .flat_map(|head| std::iter::once(head).chain(reachable[&head].iter().copied()))
                .collect()
        })
        .collect()
}

/// The columns a set of vertices needs: one, plus whatever the branchers it
/// leads with claim beyond their own single column.
///
/// Only a brancher nested inside another brancher of the set is skipped, since
/// the enclosing footprint already covers it. A brancher merely reached through
/// ordinary blocks still claims its own columns; overlooking that reserves one
/// column for a whole nested branch tree.
fn width_of(
    set: &BTreeSet<Vertex>,
    reachable: &BTreeMap<Vertex, BTreeSet<Vertex>>,
    footprints: &Footprints,
) -> usize {
    let brancher = |vertex: &Vertex| footprints.width.contains_key(&block_of(*vertex));
    let leading = set.iter().filter(|vertex| {
        brancher(vertex)
            && !set.iter().any(|other| {
                other != *vertex && brancher(other) && reachable[other].contains(vertex)
            })
    });
    1 + leading
        .map(|vertex| footprints.width[&block_of(*vertex)] - 1)
        .sum::<usize>()
}

/// The block a vertex belongs to, or `usize::MAX` for one that belongs to none.
const fn block_of(vertex: Vertex) -> usize {
    match vertex {
        Vertex::Node(NodeId::Block(block)) => block,
        _ => usize::MAX,
    }
}

/// Transitive successors of every vertex.
fn reachable(topology: &Topology) -> BTreeMap<Vertex, BTreeSet<Vertex>> {
    // ponytail: one breadth-first walk per vertex; a shared closure would pay
    // off only for diagrams far larger than a readable one.
    topology
        .vertices
        .iter()
        .map(|&start| {
            let mut seen = BTreeSet::new();
            let mut frontier = vec![start];
            while let Some(vertex) = frontier.pop() {
                for connection in topology.outgoing(vertex) {
                    let next = Vertex::from(connection.destination);
                    if seen.insert(next) {
                        frontier.push(next);
                    }
                }
            }
            (start, seen)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A topology of block nodes and the connections between them, as `rows`
    /// reads them: one exit per source, one vertex per block.
    fn linked(pairs: &[(usize, usize)]) -> Topology {
        Topology {
            nodes: vec![],
            exits: vec![],
            junctions: vec![],
            connections: pairs
                .iter()
                .map(|&(from, to)| Connection {
                    source: Source::Exit(ExitId::of(NodeId::Block(from))),
                    destination: Destination::Node(NodeId::Block(to)),
                })
                .collect(),
            vertices: pairs
                .iter()
                .flat_map(|&(from, to)| [from, to])
                .map(|block| Vertex::Node(NodeId::Block(block)))
                .collect::<BTreeSet<_>>()
                .into_iter()
                .collect(),
        }
    }

    #[test]
    fn a_cycle_is_refused_rather_than_ranked() {
        let chain = rows(&linked(&[(0, 1)]), &BTreeMap::new()).expect("a chain has an order");
        assert_eq!(chain[&Vertex::Node(NodeId::Block(0))], 1);
        assert_eq!(chain[&Vertex::Node(NodeId::Block(1))], 2);

        assert_eq!(
            rows(&linked(&[(0, 1), (1, 0)]), &BTreeMap::new()),
            Err("the connections form a cycle, so no node can be lowest".to_owned())
        );
    }
}
