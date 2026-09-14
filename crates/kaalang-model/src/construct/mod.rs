//! Constructs one conforming arrangement of a flow's visual topology, and
//! checks it independently of the search that found it.
//!
//! RFC 0002 §8 leaves exact ranks and routing space to a presentation but fixes
//! branch order, the column a shared continuation uses, crossing-free
//! orthogonal routing, and the contour of an iteration back edge. `build` asks for an
//! arrangement after semantic validation and carries it with the model, so a
//! flow whose topology has no conforming diagram is rejected there.
//!
//! # Two searches
//!
//! `preferred` looks only among the arrangements shaped the way RFC 0002 §8
//! asks for. Everything it may choose is finite and named here:
//!
//! - which iteration tails sink below every vertex precedence leaves free
//!   (`2^loops` subsets),
//! - which contour each iteration back edge takes (`2^loops` assignments),
//! - which corridor shape each connection uses (`3^connections` assignments).
//!
//! Nothing else is a choice: ranks, columns, rails, lanes, and
//! contour columns are derived from those. It follows the conflicts its own
//! planner and contour search report: each failure names a connection to give
//! more room, or a loop to move or flip, and only those assignments are tried
//! next. No assignment is visited twice, so the walk ends inside a finite
//! space; there is no attempt, time, or memory budget. Its first candidate is
//! the one RFC 0002 §8 asks for: no tail sinks, every back edge takes the contour
//! that section prefers, and every connection turns directly into its
//! destination's column.
//!
//! That family is a shape preference, not the contract, so exhausting it
//! proves nothing — and neither does one candidate of it failing the check.
//! Either way [`sweep`] then decides, and its module documents both directions
//! of why its sequences are the drawings. Only its exhaustion makes a flow
//! invalid, and only its obstruction says why.
//!
//! Running the preferred search first is not an optimization. Its results are
//! the arrangements worth drawing; the sweep's normal form spends a rank and a
//! column on every vertex, which is always drawable and rarely pretty. A
//! renderer can request a checked simplification through
//! `SemanticModel::compact_arrangement` without adding that work to `build`.

use std::collections::{BTreeMap, BTreeSet};

use syn::Error;

use crate::model::{Flow, WireMerge};
use crate::topology::{Connection, Destination, ExitId, NodeId, Topology, Vertex};

mod choice;
pub(crate) mod compact;
mod describe;
mod end;
mod loop_block;
mod place;
mod regions;
mod route;
mod sweep;
mod verify;

/// The sole arrival that keeps an ordinary vertex in its predecessor's
/// column. A case may be reached by a distributor detour, and an iteration
/// tail may finish at either end of its incoming rail.
fn serial_arrival(topology: &Topology, vertex: Vertex) -> Option<&Connection> {
    if matches!(vertex, Vertex::Node(NodeId::Case { .. }))
        || topology
            .loops
            .iter()
            .any(|loop_| vertex == Vertex::Junction(loop_.tail))
    {
        return None;
    }
    let mut incoming = topology.incoming(vertex);
    let arrival = incoming.next()?;
    incoming.next().is_none().then_some(arrival)
}

/// The vertices one loop's body draws: its own blocks and their cases, its
/// entry and tail and those of the loops nested in it, and the junctions a
/// merge or a break inside it draws.
///
/// This is what an iteration back edge climbs clear of (RFC 0002 §8), and both the
/// construction and a presentation measure the same set. The blocks alone
/// would miss the junctions, which draw no node and still occupy a column.
pub(crate) fn body_vertices(flow: &Flow, topology: &Topology, header: usize) -> BTreeSet<Vertex> {
    loop_block::body_vertices(flow, topology, header)
}

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
    /// Per connection, in `Topology::connections` order, its corridor.
    pub routes: Vec<Route>,
    /// Per rank gap, how many lanes its sideways runs occupy.
    pub gap_lanes: Vec<usize>,
    /// Per cycle, in `Topology::loops` order, the contour of its iteration back edge.
    pub contours: Vec<Contour>,
    /// Optional sideways runs of a back edge's climb, indexed by cycle. Stored in
    /// downward order from entry to tail; a renderer reverses it. An absent
    /// route is the straight climb recorded by `Contour`.
    pub back_routes: BTreeMap<usize, Route>,
}

impl Arrangement {
    /// The deepest lane any route entering one junction takes in the gap above
    /// it. Those routes meet on that lane's line, so a side route finishes
    /// horizontally on the rail rather than turning down over the continuation
    /// below it (RFC 0002 §8). `None` when none of them runs sideways there.
    #[must_use]
    pub fn deepest_lane(&self, topology: &Topology, junction: usize, gap: usize) -> Option<usize> {
        topology
            .connections
            .iter()
            .enumerate()
            .filter(|(_, wire)| wire.destination == Destination::Junction(junction))
            .flat_map(|(index, _)| &self.routes[index].runs)
            .filter(|run| run.gap == gap)
            .map(|run| run.lane)
            .max()
    }
}

/// One connection's corridor: it leaves its exit in `departure`, arrives from
/// above in `arrival`, and each of its sideways runs says which column it
/// enters that rank gap in and which it leaves by.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Route {
    pub departure: i32,
    pub arrival: i32,
    /// Sideways runs in increasing (gap, lane) order.
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

/// The side, column boundary and lane of a straight back edge's climb, or the
/// entry end of a climb with recorded runs. The boundary may stand beyond the
/// body's outermost column. A presentation puts lane 0 outside that column's
/// boxes and each later lane one step further out.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Contour {
    pub side: Side,
    pub column: i32,
    pub lane: usize,
}

/// The side of its body an iteration back edge climbs.
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
    /// The cycle whose back edge could not be drawn, when one is to blame. The
    /// search changes that loop's rank or contour next.
    pub(super) loop_index: Option<usize>,
    /// The connection in the way, when one is. The search gives that
    /// connection a longer corridor next.
    pub(super) connection: Option<usize>,
}

/// Why the preferred search returned no arrangement.
enum Preferred {
    /// Every arrangement of this shape was tried, and none conforms.
    Exhausted,
    /// The topology contradicts itself, so no shape can draw it.
    Inconsistent(String),
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

/// Constructs and independently checks one conforming arrangement, or reports
/// that the topology has no conforming diagram at all.
///
/// Two searches run in turn. `preferred` looks only among the arrangements
/// shaped the way RFC 0003 §2.5 prefers, and its results are the ones worth
/// drawing. When it finds none — or returns one the check rejects, which is a
/// defect in that search and not a fact about the topology — `sweep` decides
/// the question: it accepts exactly the topologies that can be drawn, so only
/// its exhaustion makes a flow invalid.
///
/// # Errors
///
/// Reports an impossible topology when the sweep exhausts its space, and an
/// internal construction error when the independent check rejects the
/// arrangement the sweep returned, or when the projection contradicts itself.
/// Inconsistent column constraints refuse a search state, not the whole flow.
///
/// # Panics
///
/// May panic if the inputs are not corresponding analyzed parts of one valid
/// flow, for example if the implicit end block or referenced vertices are absent.
pub(crate) fn construct(
    flow: &Flow,
    merges: &[WireMerge],
    topology: &Topology,
) -> syn::Result<Arrangement> {
    let internal = |reason| {
        Error::new(
            flow.end_span(),
            format!("internal kaalang construction error: {reason}"),
        )
    };
    match preferred(flow, merges, topology) {
        // One bad candidate says nothing about the topology, so the deciding
        // search still runs. The disagreement is a defect in the preferred
        // search all the same, and `disagreed` is what stops it being caught
        // quietly: the suite fails on it, and a release build goes on to the
        // answer that is not in doubt.
        Ok(arrangement) => match verify::arrangement(flow, topology, &arrangement) {
            Ok(()) => return Ok(arrangement),
            Err(reason) => disagreed(&reason),
        },
        Err(Preferred::Inconsistent(reason)) => return Err(internal(reason)),
        Err(Preferred::Exhausted) => {}
    }
    match sweep::search(flow, merges, topology) {
        // The sweep verifies its completed witness before returning it.
        Ok(arrangement) => Ok(arrangement),
        Err(sweep::Refusal::Internal(reason)) => Err(internal(reason)),
        Err(sweep::Refusal::Impossible(blocked)) => Err(Error::new(
            blocked.span,
            format!(
                "could not construct a diagram under RFC 0002: {}",
                blocked.message
            ),
        )),
    }
}

/// Reports that the preferred search returned an arrangement the independent
/// check rejects.
///
/// The decision does not depend on it — the deciding search runs next either
/// way — so this must not reject the flow. It is still a defect, and this
/// crate's own test run is where it is visible: the generated shapes fail on
/// it here, and a fixture the preferred search misdraws changes the diagram
/// beside it.
fn disagreed(reason: &str) {
    #[cfg(test)]
    panic!("the preferred search returned an arrangement the check rejects: {reason}");
    #[cfg(not(test))]
    let _ = reason;
}

/// Searches the arrangements shaped the way RFC 0003 §2.5 prefers, following the
/// conflicts its own planner and contour search report.
///
/// This search is incomplete: `Exhausted` says only that no arrangement of
/// this shape conforms, and the deciding sweep takes over. `Inconsistent` is a
/// topology no search can draw.
fn preferred(
    flow: &Flow,
    merges: &[WireMerge],
    topology: &Topology,
) -> Result<Arrangement, Preferred> {
    let tails = topology
        .loops
        .iter()
        .map(|loop_| Vertex::Junction(loop_.tail))
        .collect::<Vec<_>>();
    // RFC 0002 §8 prefers one contour per loop. The search starts there and
    // only flips a back edge the preferred side cannot hold.
    let sides = topology
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

    // The first state is the arrangement RFC 0002 §8 asks for: no tail sinks
    // and every back edge on the side it prefers. Each failure names the cycle to
    // blame, and the walk moves that loop's contour or rank before anything
    // else. No state is visited twice, so it ends.
    let every_tail = tails.iter().copied().collect::<BTreeSet<_>>();
    let mut pending = vec![(BTreeSet::new(), sides)];
    let mut seen = BTreeSet::new();
    while let Some((sunk, sides)) = pending.pop() {
        if !seen.insert((sunk.clone(), sides.clone())) {
            continue;
        }
        match corridors(flow, merges, topology, &sunk, &sides) {
            Ok(arrangement) => return Ok(arrangement),
            Err(Rejection::Internal(reason)) => return Err(Preferred::Inconsistent(reason)),
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
            }
        }
    }
    Err(Preferred::Exhausted)
}

/// Queues every corridor shape later than the one this connection has now.
/// Only a later rung is worth trying: each gives the run more room, and no rung
/// repeats. Pushed furthest first, so the stack hands back the nearest.
fn widen(pending: &mut Vec<Vec<Shape>>, shapes: &[Shape], connection: usize) {
    for later in Shape::ALL
        .into_iter()
        .filter(|shape| *shape > shapes[connection])
        .rev()
    {
        let mut next = shapes.to_vec();
        next[connection] = later;
        pending.push(next);
    }
}

/// The obstruction reported when the searched space ran out with nothing
/// drawable and no candidate named a participant to blame.
fn unarrangeable(flow: &Flow) -> Obstruction {
    Obstruction {
        span: flow.end_span(),
        message: "its connections cannot be arranged without a crossing".to_owned(),
        loop_index: None,
        connection: None,
    }
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
                widen(&mut pending, &shapes, connection);
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
                widen(&mut pending, &shapes, connection);
            }
            continue;
        }
        match loop_block::contours(flow, merges, topology, &arrangement, sides) {
            Ok(contours) => {
                arrangement.contours = contours;
                return Ok(arrangement);
            }
            Err(reason) => {
                // A back edge blocked by one connection may fit once that
                // connection takes a longer corridor, so the same conflict
                // that moves a contour also moves a shape.
                if let Some(connection) = reason.connection {
                    widen(&mut pending, &shapes, connection);
                }
                blocked.get_or_insert(reason);
            }
        }
    }

    Err(Rejection::Obstructed(
        blocked.unwrap_or_else(|| unarrangeable(flow)),
    ))
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
    Arrangement {
        rank: placement.rank,
        ranks: placement.ranks,
        column: placement.column,
        exit_offset,
        routes,
        gap_lanes: plan.gap_lanes.clone(),
        contours: Vec::new(),
        back_routes: BTreeMap::new(),
    }
}

#[cfg(test)]
mod tests;
