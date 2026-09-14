//! Assigns a rank and a column to every node and junction.
//!
//! RFC 0002 §8 runs execution time from top to bottom or along a side exit into
//! a merge on the same row, so a rank comes from connection reachability,
//! including the chosen serial order and the placement-only precedence a break
//! carries out of the region it leaves.
//! Branches run left to right in authored order, and a brancher reserves the
//! whole footprint its branches occupy, so that nested branchers and the
//! convergence groups they share compose without corrupting each other.
//!
//! Exact ranks are a presentation choice (RFC 0002 §8), so a vertex takes the
//! earliest rank its predecessors allow unless the search asks for it to sink
//! below everything precedence leaves free.

use std::collections::{BTreeMap, BTreeSet};

use crate::model::Flow;
use crate::topology::{Connection, ExitId, NodeId, Source, Topology, Vertex};

#[derive(Clone)]
pub(super) struct Placement {
    pub(super) rank: BTreeMap<Vertex, usize>,
    pub(super) column: BTreeMap<Vertex, i32>,
    pub(super) footprints: Footprints,
    pub(super) ranks: usize,
}

impl Placement {
    pub(super) fn row(&self, vertex: Vertex) -> usize {
        self.rank[&vertex]
    }

    pub(super) fn column(&self, vertex: Vertex) -> i32 {
        self.column[&vertex]
    }

    pub(super) fn exit_column(&self, exit: ExitId) -> i32 {
        self.column(Vertex::Node(exit.node)) + self.footprints.offset(exit)
    }
}

/// Places one projected topology, or reports that it has no top-to-bottom
/// order at all. `sunk` names the vertices to place as late as precedence
/// allows.
pub(super) fn place(
    topology: &Topology,
    flow: &Flow,
    sunk: &BTreeSet<Vertex>,
    sides: &[super::Side],
) -> Result<Placement, String> {
    let rank = rows(topology, sunk)?;
    let ranks_used = rank.values().copied().max().unwrap_or(0) + 1;
    let footprints = footprints(topology, flow);
    Ok(Placement {
        column: columns(topology, &footprints, &rank, ranks_used, sides),
        footprints,
        rank,
        ranks: ranks_used,
    })
}

/// Longest-path ranking: a node sits below every predecessor. A wire merge or a
/// sole iteration tail may share its side producer's row; other junctions sit
/// below their producers and every block ordered before them. Start owns row 0; every
/// computational node is reached from it in the chosen serial order.
///
/// Dependency depth is only the earliest row an item may take. Two further
/// passes lower items from there: the tails the caller asks to sink go below
/// everything precedence leaves free, and `separate_junctions` gives every
/// iteration tail a row to itself. Routing never asks for a row: it works with
/// the rows it is given, and reports a conflict instead when a gap cannot hold
/// its runs.
pub(super) fn rows(
    topology: &Topology,
    sunk: &BTreeSet<Vertex>,
) -> Result<BTreeMap<Vertex, usize>, String> {
    // A sunk vertex may take any rank its predecessors allow, so it goes below
    // every vertex precedence leaves free. One vertex per rank is always
    // enough room for that. Iteration tails keep their nesting while they
    // sink: an inner tail stays above the tail of the loop enclosing it, so
    // the inner arrival never spans the enclosing tail's row.
    let floor = topology.vertices.len();
    let depth = topology
        .loops
        .iter()
        .enumerate()
        .map(|(index, loop_)| {
            (
                Vertex::Junction(loop_.tail),
                topology.loops.len() - 1 - index,
            )
        })
        .collect::<BTreeMap<_, _>>();
    let mut rows = BTreeMap::new();
    let mut pending = topology.vertices.iter().copied().collect::<BTreeSet<_>>();
    while !pending.is_empty() {
        // Validated wire merges leave an acyclic visual graph, so this only
        // fails on a projection bug. Refuse the diagram rather than abort: the
        // renderer is also a library and a command-line binary.
        let Some(&ready) = pending.iter().find(|&&vertex| {
            topology
                .incoming(vertex)
                .chain(
                    topology
                        .order
                        .iter()
                        .filter(|edge| edge.destination == vertex),
                )
                .all(|connection| rows.contains_key(&Vertex::from(connection.source)))
        }) else {
            return Err("the connections form a cycle, so no node can be lowest".to_owned());
        };
        pending.remove(&ready);
        let row = topology
            .incoming(ready)
            .map(|connection| (connection, !topology.same_row_junction(connection)))
            .chain(
                topology
                    .order
                    .iter()
                    .filter(|edge| edge.destination == ready)
                    .map(|connection| (connection, true)),
            )
            .map(|(connection, descends)| {
                rows[&Vertex::from(connection.source)] + usize::from(descends)
            })
            .max()
            .unwrap_or(usize::from(ready != Vertex::Node(NodeId::Start)));
        let lowest = if sunk.contains(&ready) {
            floor + depth.get(&ready).copied().unwrap_or(0)
        } else {
            0
        };
        rows.insert(ready, row.max(lowest));
    }
    separate_junctions(topology, &mut rows);
    Ok(rows)
}

/// Gives every iteration tail a rank of its own. A back edge leaves its tail
/// horizontally, across every column between the tail and its contour, so
/// anything else on that rank stands in the way. Cycle entries keep their ranks,
/// because RFC 0002 §7 lets a cycle entry start alongside the body beside it and
/// a back edge arrives at an entry from the column immediately outside that body.
/// Splitting a rank only adds descent to forward connections and preserves
/// their order. Renumbering here also closes the ranks the longest path
/// left unused, so nothing below has to compact them.
fn separate_junctions(topology: &Topology, rows: &mut BTreeMap<Vertex, usize>) {
    let alone = topology
        .loops
        .iter()
        .map(|loop_| Vertex::Junction(loop_.tail))
        .collect::<BTreeSet<_>>();
    let mut ranks: BTreeMap<usize, Vec<Vertex>> = BTreeMap::new();
    for (&vertex, &rank) in rows.iter() {
        ranks.entry(rank).or_default().push(vertex);
    }
    let mut next = 0;
    for (_, vertices) in ranks {
        let (isolated, shared): (Vec<Vertex>, Vec<Vertex>) = vertices
            .into_iter()
            .partition(|vertex| alone.contains(vertex));
        if !shared.is_empty() {
            for vertex in shared {
                rows.insert(vertex, next);
            }
            next += 1;
        }
        for vertex in isolated {
            rows.insert(vertex, next);
            next += 1;
        }
    }
}

/// The columns of one brancher's branches, as offsets from its own column. The
/// first branch continues down the brancher's column and later branches sit to
/// its right, in authored order.
#[derive(Clone)]
pub(super) struct Footprints {
    /// Branch offsets per branching block.
    offsets: BTreeMap<usize, Vec<usize>>,
    /// Total columns owned per branching block.
    width: BTreeMap<usize, usize>,
}

impl Footprints {
    pub(super) fn offset(&self, exit: ExitId) -> i32 {
        match (exit.node, exit.branch) {
            (NodeId::Block(block), Some(branch)) => self.offsets[&block][branch] as i32,
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
    sides: &[super::Side],
) -> BTreeMap<Vertex, i32> {
    let mut ordered = topology.vertices.clone();
    ordered.sort_by_key(|vertex| (rows[vertex], *vertex));

    let mut columns = BTreeMap::new();
    let mut occupied = vec![BTreeSet::new(); rows_used];
    for vertex in ordered {
        let Vertex::Node(node) = vertex else {
            // A junction owns no column of its own; it is placed once the
            // consumers its common segment reaches have theirs.
            continue;
        };
        let row = rows[&vertex];
        let mut column = preferred(topology, &columns, footprints, node);
        let width = footprints.span(node) as i32;
        // A later-authored continuation may occupy free columns to the left
        // of a cycle body already placed on this row.
        while (column..column + width).any(|slot| occupied[row].contains(&slot)) {
            column += 1;
        }
        occupied[row].extend(column..column + width);
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
        let preferred = if let Some(arrival) = super::serial_arrival(topology, vertex) {
            arrives_from(topology, &columns, footprints, arrival)
        } else if let Some(index) = topology
            .loops
            .iter()
            .position(|loop_| loop_.tail == junction)
        {
            let arrivals = topology
                .incoming(vertex)
                .map(|connection| arrives_from(topology, &columns, footprints, connection));
            // The back edge leaves the end of the rail nearest the contour it
            // takes, without turning back over its incoming branches.
            if sides.get(index).copied() == Some(super::Side::Right) {
                arrivals.max()
            } else {
                arrivals.min()
            }
            .unwrap_or(0)
        } else if topology.junctions[junction].is_break {
            topology
                .incoming(vertex)
                .map(|connection| arrives_from(topology, &columns, footprints, connection))
                .min()
                .unwrap_or(0)
        } else {
            topology
                .outgoing(vertex)
                .filter_map(|connection| columns.get(&connection.destination).copied())
                .min()
                .unwrap_or(0)
        };
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
    columns: &BTreeMap<Vertex, i32>,
    footprints: &Footprints,
    node: NodeId,
) -> i32 {
    if let NodeId::Case { choice, branch } = node {
        let select = columns[&Vertex::Node(NodeId::Block(choice))];
        return select + footprints.offsets[&choice][branch] as i32;
    }

    // RFC 0002 §8 puts branches in authored order left to right, so a branch's
    // own column decides where its successor goes; only a node no branch
    // reaches falls back to its leftmost predecessor, and a root to column 0.
    let arrival = |connection| arrives_from(topology, columns, footprints, connection);
    let branches = topology
        .incoming(Vertex::Node(node))
        .filter(|connection| from_branch(connection.source))
        .map(arrival)
        .min();

    branches.unwrap_or_else(|| {
        topology
            .incoming(Vertex::Node(node))
            .map(arrival)
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
    columns: &BTreeMap<Vertex, i32>,
    footprints: &Footprints,
    connection: &Connection,
) -> i32 {
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

fn footprints(topology: &Topology, flow: &Flow) -> Footprints {
    let reachable = super::regions::reachable(topology);
    let mut footprints = Footprints {
        offsets: BTreeMap::new(),
        width: BTreeMap::new(),
    };
    // Deepest first, so a nested brancher's width is known before the brancher
    // whose branch contains it asks for it.
    let mut ordered = super::regions::branchers(flow, topology);
    ordered.sort_by_key(|&block| reachable[&Vertex::Node(NodeId::Block(block))].len());
    for block in ordered {
        let branches = super::regions::branch_sets(topology, flow, &reachable, block);
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
