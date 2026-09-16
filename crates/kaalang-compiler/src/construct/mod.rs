//! Constructs and independently verifies an arrangement under RFC 0002 §8.
//!
//! `preferred` seeks readable diagrams by varying sunk tails, contour sides,
//! and corridor shapes. Conflicts guide a finite search with no repeated states
//! or resource cutoff; all other placement and routing choices are derived.
//! If it exhausts its candidates or verification fails, `sweep` decides
//! realizability. Only sweep exhaustion proves the topology impossible.
//!
//! See RFC 0003 §2.1 for the complete search and §2.5 for presentation choices.
//! Rendering may compact the verified arrangement; macro compilation skips this.

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

/// Body vertices shared by construction and rendering, including nested cycles
/// and structural junctions that occupy columns without drawing nodes.
pub(crate) fn body_vertices(flow: &Flow, topology: &Topology, header: usize) -> BTreeSet<Vertex> {
    loop_block::body_vertices(flow, topology, header)
}

/// A checked arrangement of one topology. Ranks and columns are abstract
/// integers: a presentation assigns dimensions and spacing to them, and may not
/// reorder or re-route anything recorded here.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Arrangement {
    /// Abstract row of every vertex, checked against RFC 0002 §8.
    pub rank: BTreeMap<Vertex, usize>,
    /// One past the deepest rank in use.
    pub ranks: usize,
    /// Abstract column of every vertex.
    pub column: BTreeMap<Vertex, i32>,
    /// The default branch column each exit owns, as an offset from its node's
    /// column. A select distributor may also leave by a destination case's column.
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
    /// Every corridor: each connection's route, then each recorded back edge's.
    pub(crate) fn all_routes(&self) -> impl Iterator<Item = &Route> {
        self.routes.iter().chain(self.back_routes.values())
    }

    pub(crate) fn all_routes_mut(&mut self) -> impl Iterator<Item = &mut Route> {
        self.routes.iter_mut().chain(self.back_routes.values_mut())
    }
}

/// One connection's corridor: it leaves its exit in `departure`, reaches its
/// destination in `arrival`, and each sideways run records its horizontal line
/// and the columns it joins.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Route {
    pub departure: i32,
    pub arrival: i32,
    /// Sideways runs in drawing order, each on its recorded rank or gap lane.
    pub runs: Vec<Run>,
}

/// The horizontal line occupied by a sideways run.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RunLine {
    /// The centre of a vertex row, including a visible merge's arrival rail.
    Rank(usize),
    /// A routing lane in the gap below a rank.
    Lane { gap: usize, lane: usize },
}

/// One sideways run of a corridor, on a rank or a lane between ranks.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Run {
    pub line: RunLine,
    pub enter: i32,
    pub exit: i32,
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

/// Alternative corridor shapes, tried in this order.
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

/// Deduplicates equivalent shapes: junction arrivals keep separate descents
/// until the merge rail, making `Direct` and `Deferred` identical.
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

/// Runs the preferred search, falling back to the complete sweep.
///
/// # Errors
///
/// Reports sweep exhaustion as impossible topology; contradictory projection
/// or a failed sweep verification as an internal error.
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

/// Fails unit tests on a preferred-search defect; other builds fall back to the sweep.
fn disagreed(reason: &str) {
    #[cfg(test)]
    panic!("the preferred search returned an arrangement the check rejects: {reason}");
    #[cfg(not(test))]
    let _ = reason;
}

/// Conflict-guided search of the preferred shapes (RFC 0003 §2.5).
/// Exhaustion requires a sweep; inconsistency indicates a projection defect.
fn preferred(
    flow: &Flow,
    merges: &[WireMerge],
    topology: &Topology,
) -> Result<Arrangement, Preferred> {
    // Branch widths depend on topology, not the ranks or contours tried below.
    let footprints = place::footprints(topology, flow);
    let tails = topology
        .loops
        .iter()
        .map(|loop_| Vertex::Junction(loop_.tail))
        .collect::<Vec<_>>();
    // Start with the contour preference from RFC 0002 §8.
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

    let every_tail = tails.iter().copied().collect::<BTreeSet<_>>();
    let mut pending = vec![(BTreeSet::new(), sides)];
    let mut seen = BTreeSet::new();
    while let Some((sunk, sides)) = pending.pop() {
        if !seen.insert((sunk.clone(), sides.clone())) {
            continue;
        }
        match corridors(flow, merges, topology, &footprints, &sunk, &sides) {
            Ok(arrangement) => return Ok(arrangement),
            Err(sweep::Refusal::Internal(reason)) => return Err(Preferred::Inconsistent(reason)),
            Err(sweep::Refusal::Impossible(reason)) => {
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

/// Queues wider shapes in reverse order so the stack tries the nearest first.
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

/// Resolves corridors for fixed ranks and contour sides, widening a conflicting
/// route at each step. Each shape assignment is visited at most once.
fn corridors(
    flow: &Flow,
    merges: &[WireMerge],
    topology: &Topology,
    footprints: &place::Footprints,
    sunk: &BTreeSet<Vertex>,
    sides: &[Side],
) -> Result<Arrangement, sweep::Refusal> {
    let placement =
        place::place(topology, footprints, sunk, sides).map_err(sweep::Refusal::Internal)?;
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
                // Widening the blocking route may free the contour.
                if let Some(connection) = reason.connection {
                    widen(&mut pending, &shapes, connection);
                }
                blocked.get_or_insert(reason);
            }
        }
    }

    Err(sweep::Refusal::Impossible(
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
                    line: RunLine::Lane {
                        gap: *gap,
                        lane: plan.lanes[&(index, *gap)],
                    },
                    enter: crossing.enter,
                    exit: crossing.exit,
                })
                .chain(plan.arrivals.get(&index).copied())
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
