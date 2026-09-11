//! Constructs one conforming arrangement of a flow's visual topology, and
//! checks it independently of the search that found it.
//!
//! RFC 0002 §8 leaves exact ranks and routing space to a presentation but fixes
//! branch order, the column a shared continuation uses, crossing-free
//! orthogonal routing, and the contour of a loop return. A renderer requests an
//! arrangement after semantic validation and realizes it if construction succeeds.
//! ponytail: this finite heuristic search has no completeness proof; keep failure
//! in the renderer until normalization and conflict pruning are proved complete.
//!
//! # The arrangement space
//!
//! Everything the search may choose is finite and named here:
//!
//! - which iteration tails sink below every vertex precedence leaves free
//!   (`2^loops` subsets),
//! - which contour each loop return takes (`2^loops` assignments),
//! - which corridor shape each connection uses (`3^connections` assignments).
//!
//! Nothing else is a choice: ranks, columns, footprints, rails, lanes, and
//! contour columns are derived from those. The search follows the conflicts its
//! own planner and contour search report: each failure names a connection to
//! give more room, or a loop to move or flip, and only those assignments are
//! tried next. No assignment is visited twice, so the walk ends inside a finite
//! space; there is no attempt, time, or memory budget.
//!
//! The first candidate is the one RFC 0002 §8 asks for: no tail sinks, every
//! return takes the contour that section prefers, and every connection turns
//! directly into its destination's column. A later candidate is reached only
//! because that one could not be drawn.

use std::collections::{BTreeMap, BTreeSet};

use syn::Error;

use crate::model::{Flow, WireMerge};
use crate::topology::{Destination, ExitId, NodeId, Topology, Vertex};

mod describe;
mod loop_block;
mod place;
mod regions;
mod route;
mod verify;

/// A checked arrangement of one topology. Ranks and columns are abstract
/// integers: a presentation assigns dimensions and spacing to them, and may not
/// reorder or re-route anything recorded here.
#[derive(Clone, Default)]
pub struct Arrangement {
    /// Abstract row of every vertex. A forward connection descends, and so does
    /// every placement-only precedence edge.
    pub rank: BTreeMap<Vertex, usize>,
    /// One past the deepest rank in use.
    pub ranks: usize,
    /// Abstract column of every vertex.
    pub column: BTreeMap<Vertex, i32>,
    /// The column each exit leaves by, as an offset from its node's column.
    pub exit_offset: BTreeMap<ExitId, i32>,
    /// Per branching block, the columns its branches and their shared
    /// continuations reserve (RFC 0002 §8).
    pub footprints: BTreeMap<usize, Footprint>,
    /// Per connection, in `Topology::connections` order, its corridor.
    pub routes: Vec<Route>,
    /// Per rank gap, how many lanes its sideways runs occupy.
    pub gap_lanes: Vec<usize>,
    /// Per loop, in `Topology::loops` order, the contour of its return.
    pub contours: Vec<Contour>,
}

/// The branch columns of one question or choice, as offsets from its own
/// column, and the total number of columns it reserves.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Footprint {
    pub offsets: Vec<usize>,
    pub width: usize,
}

/// One connection's corridor: it leaves its exit in `departure`, arrives from
/// above in `arrival`, and each of its sideways runs says which column it
/// enters that rank gap in and which it leaves by.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Route {
    pub departure: i32,
    pub arrival: i32,
    /// The sideways runs it makes, one per rank gap that needs one.
    pub runs: Vec<Run>,
}

/// One sideways run of a corridor, inside one rank gap.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Run {
    pub gap: usize,
    pub enter: i32,
    pub exit: i32,
    pub lane: usize,
}

/// The contour one loop return climbs: the side of its body, the outermost
/// column of that body, and which lane beside it the return takes. A
/// presentation puts lane 0 immediately outside that column's node and each
/// later lane one step further out.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Contour {
    pub side: Side,
    pub column: i32,
    pub lane: usize,
}

/// The side of its body a loop return climbs.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Side {
    Left,
    Right,
}

/// How a connection reaches its destination's column. The three are
/// alternatives, not stages: the search replaces one with another rather than
/// adding to it, so the shape that works is never overridden by a later one.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum Shape {
    /// Turn sideways in the gap below the source and descend the rest of the
    /// way in the destination's column.
    #[default]
    Direct,
    /// Descend in the source's column and turn sideways in the gap above the
    /// destination.
    Deferred,
    /// Leave the diagram on the side it is heading and come back, clear of
    /// everything in between.
    Aside,
}

impl Shape {
    const ALL: [Self; 3] = [Self::Direct, Self::Deferred, Self::Aside];
}

/// Collapses the shapes that name the same corridor, so the walk does not
/// visit one assignment under two names.
///
/// A connection reaching a junction descends in its own column whichever of
/// `Direct` and `Deferred` it is given: producer branches keep separate
/// descents until the merge rail.
fn normalize(topology: &Topology, shapes: &mut [Shape]) {
    for (index, shape) in shapes.iter_mut().enumerate() {
        if *shape == Shape::Deferred
            && matches!(
                topology.connections[index].destination,
                Destination::Junction(_)
            )
        {
            *shape = Shape::Direct;
        }
    }
}

/// One reason a candidate arrangement does not conform, at the block it
/// concerns.
pub(super) struct Obstruction {
    pub(super) span: proc_macro2::Span,
    pub(super) message: String,
    /// The loop whose return could not be drawn, when one is to blame. The
    /// search changes that loop's rank or contour next.
    pub(super) loop_index: Option<usize>,
    /// The connection in the way, when one is. The search gives that
    /// connection a longer corridor next.
    pub(super) connection: Option<usize>,
}

/// Why one rank and contour assignment yielded no arrangement.
enum Rejection {
    /// The arrangement cannot be drawn. The search moves to another one.
    Obstructed(Obstruction),
    /// The topology itself is inconsistent, so no assignment of it can be
    /// drawn and trying another is pointless. A projection bug, not an authored
    /// flow error.
    Internal(String),
}

/// Attempts to construct and check an arrangement from analyzed flow parts.
///
/// This search is incomplete. Failure does not make the flow invalid, and the
/// caller must not use it as a semantic acceptance check.
///
/// # Errors
///
/// Reports an obstruction when no searched candidate conforms, or an internal
/// construction error when the independent verifier rejects a returned candidate.
///
/// # Panics
///
/// May panic if the inputs are not corresponding analyzed parts of one valid
/// flow, for example if the implicit end block or referenced vertices are absent.
pub fn construct(
    flow: &Flow,
    merges: &[WireMerge],
    topology: &Topology,
) -> syn::Result<Arrangement> {
    let internal = |reason| {
        Error::new(
            flow.blocks
                .last()
                .expect("a flow owns the implicit end block")
                .span,
            format!("internal kaalang construction error: {reason}"),
        )
    };
    let tails = topology
        .loops
        .iter()
        .map(|loop_| Vertex::Junction(loop_.tail))
        .collect::<Vec<_>>();
    // RFC 0002 §8 prefers one contour per loop. The search starts there and
    // only flips a return the preferred side cannot hold.
    let preferred = topology
        .loops
        .iter()
        .map(|loop_| {
            if loop_.prefer_left {
                Side::Left
            } else {
                Side::Right
            }
        })
        .collect::<Vec<_>>();

    // The obstruction reported is the first one found, which belongs to the
    // arrangement closest to what RFC 0002 §8 asks for. Later states are
    // further from it, so their diagnostics describe an arrangement the author
    // never asked for.
    let mut blocked = None;
    // The first state is the arrangement RFC 0002 §8 asks for: no tail sinks
    // and every return on the side it prefers. Each failure names the loop to
    // blame, and the walk moves that loop's contour or rank before anything
    // else. No state is visited twice, so it ends.
    let every_tail = tails.iter().copied().collect::<BTreeSet<_>>();
    let mut pending = vec![(BTreeSet::new(), preferred.clone())];
    let mut seen = BTreeSet::new();
    while let Some((sunk, sides)) = pending.pop() {
        if !seen.insert((sunk.clone(), sides.clone())) {
            continue;
        }
        match corridors(flow, merges, topology, &sunk, &sides) {
            Ok(arrangement) => {
                return match verify::arrangement(flow, topology, &arrangement) {
                    Ok(()) => Ok(arrangement),
                    Err(reason) => Err(internal(reason)),
                };
            }
            Err(Rejection::Internal(reason)) => return Err(internal(reason)),
            Err(Rejection::Obstructed(reason)) => {
                if let Some(index) = reason.loop_index {
                    // Pushed in reverse order of preference: flipping one
                    // contour is tried before lowering one tail, and lowering
                    // every tail is the last resort.
                    pending.push((every_tail.clone(), sides.clone()));
                    let mut lowered = sunk.clone();
                    lowered.insert(tails[index]);
                    pending.push((lowered, sides.clone()));
                    let mut flipped = sides.clone();
                    flipped[index] = match sides[index] {
                        Side::Left => Side::Right,
                        Side::Right => Side::Left,
                    };
                    pending.push((sunk.clone(), flipped));
                }
                blocked.get_or_insert(reason);
            }
        }
    }

    let blocked = blocked.unwrap_or_else(|| Obstruction {
        span: flow
            .blocks
            .last()
            .expect("a flow owns the implicit end block")
            .span,
        message: "its connections cannot be arranged without a crossing".to_owned(),
        loop_index: None,
        connection: None,
    });
    Err(Error::new(
        blocked.span,
        format!(
            "could not construct a diagram under RFC 0002: {}",
            blocked.message
        ),
    ))
}

/// Resolves the corridor shapes for one rank and contour assignment, by moving
/// a participant of every conflict the planner reports. Each assignment is
/// visited at most once, so the walk ends.
fn corridors(
    flow: &Flow,
    merges: &[WireMerge],
    topology: &Topology,
    sunk: &BTreeSet<Vertex>,
    sides: &[Side],
) -> Result<Arrangement, Rejection> {
    let placement =
        place::place(topology, flow, merges, sunk, sides).map_err(Rejection::Internal)?;
    let count = topology.connections.len();
    let mut pending = vec![vec![Shape::default(); count]];
    let mut seen = BTreeSet::new();
    let mut blocked = None;
    while let Some(mut shapes) = pending.pop() {
        normalize(topology, &mut shapes);
        if !seen.insert(shapes.clone()) {
            continue;
        }
        let plan = match route::plan(flow, merges, topology, &placement, &shapes) {
            Ok(plan) => plan,
            Err(route::Blocked { connection, reason }) => {
                blocked.get_or_insert(reason);
                // Only a later rung is worth trying: every rung after the
                // current one gives the run more room, and no rung repeats.
                // Pushed furthest first, so the stack hands back the nearest.
                for later in Shape::ALL
                    .into_iter()
                    .filter(|shape| *shape > shapes[connection])
                    .rev()
                {
                    let mut next = shapes.clone();
                    next[connection] = later;
                    pending.push(next);
                }
                continue;
            }
        };
        let mut arrangement = assemble(topology, placement.clone(), &plan);
        if let Some((left, right)) = route::crossing(topology, &arrangement) {
            blocked.get_or_insert_with(|| {
                route::obstruction(
                    flow,
                    topology,
                    left,
                    format!(
                        "{} crosses {}",
                        describe::connection(flow, merges, topology, left),
                        describe::connection(flow, merges, topology, right),
                    ),
                )
            });
            // Shared destinations allow collinear bundles, not arbitrary
            // crossings. Check complete routes, including side departures on
            // node rows, before choosing contours. Either participant may move.
            for connection in [right, left] {
                for later in Shape::ALL
                    .into_iter()
                    .filter(|shape| *shape > shapes[connection])
                    .rev()
                {
                    let mut next = shapes.clone();
                    next[connection] = later;
                    pending.push(next);
                }
            }
            continue;
        }
        match loop_block::contours(flow, merges, topology, &arrangement, sides) {
            Ok(contours) => {
                arrangement.contours = contours;
                return Ok(arrangement);
            }
            Err(reason) => {
                // A return blocked by one connection may fit once that
                // connection takes a longer corridor, so the same conflict
                // that moves a contour also moves a shape.
                if let Some(connection) = reason.connection {
                    for later in Shape::ALL
                        .into_iter()
                        .filter(|shape| *shape > shapes[connection])
                        .rev()
                    {
                        let mut next = shapes.clone();
                        next[connection] = later;
                        pending.push(next);
                    }
                }
                blocked.get_or_insert(reason);
            }
        }
    }

    Err(Rejection::Obstructed(blocked.unwrap_or_else(|| {
        Obstruction {
            span: flow
                .blocks
                .last()
                .expect("a flow owns the implicit end block")
                .span,
            message: "its connections cannot be arranged without a crossing".to_owned(),
            loop_index: None,
            connection: None,
        }
    })))
}

/// Collects the chosen placement and corridors into one record. The contours
/// are decided against it and filled in afterwards.
fn assemble(topology: &Topology, placement: place::Placement, plan: &route::Plan) -> Arrangement {
    let routes = topology
        .connections
        .iter()
        .enumerate()
        .map(|(index, wire)| Route {
            departure: route::departure_column(&placement, wire.source, wire.destination),
            arrival: placement.column(wire.destination),
            runs: plan.crossings[index]
                .iter()
                .filter(|(_, crossing)| crossing.sideways())
                .map(|(gap, crossing)| Run {
                    gap: *gap,
                    enter: crossing.enter,
                    exit: crossing.exit,
                    lane: plan.lanes[&(index, *gap)],
                })
                .collect(),
        })
        .collect();
    let exit_offset = topology
        .exits
        .iter()
        .map(|exit| (exit.id, placement.footprints.offset(exit.id)))
        .collect();
    let mut footprints = BTreeMap::new();
    for node in &topology.nodes {
        if let NodeId::Block(block) = node.id
            && let (Some(offsets), Some(width)) = (
                placement.footprints.branch_offsets(block),
                placement.footprints.total(block),
            )
        {
            footprints.insert(
                block,
                Footprint {
                    offsets: offsets.clone(),
                    width,
                },
            );
        }
    }

    Arrangement {
        rank: placement.rank,
        ranks: placement.ranks,
        column: placement.column,
        exit_offset,
        footprints,
        routes,
        gap_lanes: plan.gap_lanes.clone(),
        contours: Vec::new(),
    }
}

#[cfg(test)]
mod tests;
