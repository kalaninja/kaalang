//! Chooses the corridor every connection runs in, and the order of the
//! sideways runs that share one rank gap.
//!
//! A route descends in its exit's column, except that a question's side exit
//! may join a merge column immediately and a select's distributor leaves
//! sideways in each later case's column. It crosses each rank gap it needs
//! sideways in one lane and enters its destination from above. Two connections
//! may share a run only when they leave one exit or reach one destination,
//! which is what draws a fan-out, a select's distributor, and a wire merge as
//! one bundle rather than as routes hidden behind each other (RFC 0002 §8).
//!
//! The lanes of one rank gap are not guessed. A horizontal run that passes over
//! another route's descent must lie below it, and one that passes over another
//! route's arrival must lie above it; those requirements form a partial order,
//! and its topological order is the lane order. A cycle means the gap has no
//! crossing-free arrangement in this corridor assignment, and the search then
//! moves the run the cycle blames.

use std::collections::{BTreeMap, BTreeSet};

use crate::model::{Flow, WireMerge};
use crate::topology::{Destination, NodeId, Source, Topology, Vertex};

use super::place::Placement;
use super::{Run, RunLine, Shape, describe};

/// What one connection does in one rank gap: it enters from above at `enter`,
/// leaves downward at `exit`, and needs a lane when those differ.
#[derive(Clone, Copy)]
pub(super) struct Crossing {
    pub(super) connection: usize,
    pub(super) enter: i32,
    pub(super) exit: i32,
}

impl Crossing {
    pub(super) const fn sideways(&self) -> bool {
        self.enter != self.exit
    }

    /// Whether a vertical at `column` would meet this run. The ends count: a
    /// run that stops on another route's descent touches it just as surely as
    /// one that passes over it.
    const fn spans(&self, column: i32) -> bool {
        let (left, right) = if self.enter < self.exit {
            (self.enter, self.exit)
        } else {
            (self.exit, self.enter)
        };
        column >= left && column <= right
    }
}

/// The planned corridors of one placement.
pub(super) struct Plan {
    /// Per connection, the gaps it crosses and what it does in each.
    pub(super) crossings: Vec<Vec<(usize, Crossing)>>,
    /// Lane of each sideways run, keyed by connection and gap.
    pub(super) lanes: BTreeMap<(usize, usize), usize>,
    pub(super) gap_lanes: Vec<usize>,
    /// Final sideways arrivals on junction rows, outside the rank gaps.
    pub(super) arrivals: BTreeMap<usize, Run>,
}

/// Which connection the search has to move when one rank gap has no
/// crossing-free arrangement.
pub(super) struct Blocked {
    pub(super) connection: usize,
    /// What could not be drawn, for the diagnostic when nothing fits.
    pub(super) reason: super::Obstruction,
}

/// Plans the rank gaps every connection crosses and the lane each sideways run
/// takes, from columns alone.
pub(super) fn plan(
    flow: &Flow,
    merges: &[WireMerge],
    topology: &Topology,
    placement: &Placement,
    shapes: &[Shape],
) -> Result<Plan, Blocked> {
    // Where a descent may not pass. A junction counts: the routes that meet
    // there occupy its point just as a node occupies its box.
    let taken = topology
        .vertices
        .iter()
        .map(|&vertex| (placement.row(vertex), placement.column(vertex)))
        .collect::<BTreeSet<_>>();

    let spans = topology
        .connections
        .iter()
        .enumerate()
        .map(|(index, wire)| {
            let source = Vertex::from(wire.source);
            let destination = wire.destination;
            let shape = shapes[index];
            // A descent through an intermediate rank needs a column free of
            // nodes there. Try the arrival column, then the departure column,
            // before allocating a separate column for an actual obstacle.
            let top = placement.row(source);
            let bottom = placement.row(destination);
            let free_of_nodes =
                |column| (top + 1..bottom).all(|row| !taken.contains(&(row, column)));
            let start = departure_column(placement, wire.source, wire.destination);
            let end = placement.column(destination);
            let waypoint = match shape {
                Shape::Aside => None,
                // Producer branches keep separate descents until the merge rail.
                _ if matches!(wire.destination, Destination::Junction(_)) => {
                    free_of_nodes(start).then_some(start)
                }
                Shape::Deferred => free_of_nodes(start).then_some(start),
                Shape::Direct => free_of_nodes(end)
                    .then_some(end)
                    .or_else(|| free_of_nodes(start).then_some(start)),
            };
            (top, bottom, start, end, waypoint)
        })
        .collect::<Vec<_>>();

    let own = own_columns(topology, placement, shapes, &taken, &spans);
    let descents = spans
        .iter()
        .enumerate()
        .map(|(index, &(.., waypoint))| waypoint.unwrap_or_else(|| own[&index]))
        .collect::<Vec<_>>();

    let arrivals = spans
        .iter()
        .enumerate()
        .filter_map(|(index, &(top, bottom, start, end, _))| {
            let enter = if top == bottom {
                start
            } else {
                descents[index]
            };
            (matches!(
                topology.connections[index].destination,
                Destination::Junction(_)
            ) && enter != end)
                .then_some((
                    index,
                    Run {
                        line: RunLine::Rank(bottom),
                        enter,
                        exit: end,
                    },
                ))
        })
        .collect::<BTreeMap<_, _>>();

    let crossings = spans
        .iter()
        .enumerate()
        .map(|(index, &(top, bottom, start, end, _))| {
            let waypoint = descents[index];
            (top..bottom)
                .map(|gap| {
                    let crossing = Crossing {
                        connection: index,
                        enter: if gap == top { start } else { waypoint },
                        exit: if gap + 1 == bottom && !arrivals.contains_key(&index) {
                            end
                        } else {
                            waypoint
                        },
                    };
                    (gap, crossing)
                })
                .collect()
        })
        .collect();

    let mut plan = Plan {
        crossings,
        lanes: BTreeMap::new(),
        gap_lanes: vec![0; placement.ranks],
        arrivals,
    };
    assign_lanes(flow, merges, topology, &mut plan)?;

    Ok(plan)
}

/// A column of its own for every route that cannot descend in the column it
/// arrives at or departs from.
///
/// They are handed out in order of the column each route departs from, and each
/// takes the leftmost column that is free over its whole rank span, starting
/// from the leftmost of its own two ends. An `Aside` route is the exception: it
/// was sent out of another route's way, so it leaves the diagram on the side it
/// is already heading — a negative column when it heads left, and one past the
/// widest column when it heads right.
fn own_columns(
    topology: &Topology,
    placement: &Placement,
    shapes: &[Shape],
    taken: &BTreeSet<(usize, i32)>,
    spans: &[(usize, usize, i32, i32, Option<i32>)],
) -> BTreeMap<usize, i32> {
    let mut claimed: BTreeSet<(usize, i32)> = BTreeSet::new();
    let mut aside: BTreeSet<(usize, i32)> = BTreeSet::new();
    let outside = taken
        .iter()
        .map(|(_, column)| column + 1)
        .max()
        .unwrap_or_default();
    let mut own = spans
        .iter()
        .enumerate()
        .filter(|(_, (.., waypoint))| waypoint.is_none())
        .map(|(index, &(top, bottom, start, ..))| (start, index, top, bottom))
        .collect::<Vec<_>>();
    own.sort_unstable();

    let mut columns = BTreeMap::new();
    for (_, index, top, bottom) in own {
        let (.., start, end, _) = spans[index];
        // The search sent an Aside route out of another's way, so it leaves the
        // diagram on the side it is already heading: a run that turned back
        // across every column would swallow the runs it was avoiding.
        let inside = shapes[index] != Shape::Aside;
        let lanes = if inside { &mut claimed } else { &mut aside };
        let mut lane = if inside {
            placement
                .column(Vertex::from(topology.connections[index].source))
                .min(placement.column(topology.connections[index].destination))
        } else {
            0
        };
        while (top..=bottom)
            .any(|row| lanes.contains(&(row, lane)) || (inside && taken.contains(&(row, lane))))
        {
            lane += 1;
        }
        for row in top..=bottom {
            lanes.insert((row, lane));
        }
        columns.insert(
            index,
            if inside {
                lane
            } else if end < start {
                -(lane + 1)
            } else {
                outside + lane
            },
        );
    }

    columns
}

/// Orders the sideways runs of each rank gap so that none meets a descent or an
/// arrival it is not allowed to meet.
fn assign_lanes(
    flow: &Flow,
    merges: &[WireMerge],
    topology: &Topology,
    plan: &mut Plan,
) -> Result<(), Blocked> {
    let mut gaps: BTreeMap<usize, Vec<Crossing>> = BTreeMap::new();
    for crossings in &plan.crossings {
        for (gap, crossing) in crossings {
            gaps.entry(*gap).or_default().push(*crossing);
        }
    }

    for (gap, crossings) in gaps {
        let sideways = crossings
            .iter()
            .filter(|crossing| crossing.sideways())
            .collect::<Vec<_>>();
        let groups = merge_lanes(topology, plan, gap, &sideways);
        let above = requirements(flow, merges, topology, &crossings, &sideways, &groups)?;
        assign_gap(
            flow, merges, topology, plan, gap, &sideways, &groups, &above,
        )?;
    }

    Ok(())
}

/// Which run has to lie above which inside one rank gap.
fn requirements(
    flow: &Flow,
    merges: &[WireMerge],
    topology: &Topology,
    crossings: &[Crossing],
    sideways: &[&Crossing],
    groups: &[usize],
) -> Result<Vec<Vec<usize>>, Blocked> {
    // Constraints belong to the group's first run. Other group members
    // occupy that same lane rather than drawing a second merge rail.
    let mut above: Vec<Vec<usize>> = vec![Vec::new(); sideways.len()];
    for (a, run) in sideways.iter().enumerate() {
        let a = groups[a];
        for other in crossings {
            if other.connection == run.connection
                || bundled(topology, run.connection, other.connection)
            {
                continue;
            }
            let over_descent = run.spans(other.enter);
            let over_arrival = run.spans(other.exit);
            let Some(b) = sideways
                .iter()
                .position(|candidate| candidate.connection == other.connection)
            else {
                // The other route passes straight through this gap, so no
                // lane order can move it out of the way; send it down a
                // column of its own instead.
                if over_descent || over_arrival {
                    return Err(Blocked {
                        connection: mover(topology, other.connection, run.connection),
                        reason: obstruction(
                            flow,
                            topology,
                            run.connection,
                            format!(
                                "{} runs across {}, which passes through the same row",
                                describe::connection(flow, merges, topology, run.connection),
                                describe::connection(flow, merges, topology, other.connection)
                            ),
                        ),
                    });
                }
                continue;
            };
            let b = groups[b];
            // Passing over another run's descent means lying below it;
            // passing over its arrival means lying above it. If both ends
            // lie inside this run's span, the other route must leave it.
            if over_descent && over_arrival {
                return Err(Blocked {
                    connection: mover(topology, other.connection, run.connection),
                    reason: obstruction(
                        flow,
                        topology,
                        run.connection,
                        format!(
                            "{} runs across both ends of {} in one row",
                            describe::connection(flow, merges, topology, run.connection),
                            describe::connection(flow, merges, topology, other.connection)
                        ),
                    ),
                });
            }
            if over_descent {
                above[a].push(b);
            }
            if over_arrival {
                above[b].push(a);
            }
        }
    }
    Ok(above)
}

/// Places the runs of one rank gap on lanes, lowest first.
#[allow(clippy::too_many_arguments)]
fn assign_gap(
    flow: &Flow,
    merges: &[WireMerge],
    topology: &Topology,
    plan: &mut Plan,
    gap: usize,
    sideways: &[&Crossing],
    groups: &[usize],
    above: &[Vec<usize>],
) -> Result<(), Blocked> {
    let ordered =
        order(above, &|run| movable(topology, sideways[run].connection)).map_err(|run| {
            Blocked {
                connection: sideways[run].connection,
                reason: obstruction(
                    flow,
                    topology,
                    sideways[run].connection,
                    format!(
                        "{} cannot share a row with the routes it meets there",
                        describe::connection(flow, merges, topology, sideways[run].connection)
                    ),
                ),
            }
        })?;
    let mut lanes = BTreeMap::new();
    for run in ordered {
        if groups[run] == run {
            // Only an ordering constraint requires another level. Runs
            // that can share a horizontal should not form a staircase.
            let lane = above[run]
                .iter()
                .map(|earlier| lanes[earlier] + 1)
                .max()
                .unwrap_or(0);
            lanes.insert(run, lane);
        }
    }
    for (run, group) in sideways.iter().zip(groups.iter().copied()) {
        plan.lanes.insert((run.connection, gap), lanes[&group]);
    }
    // The gap needs room for the levels actually occupied, not for one per
    // group: runs no ordering separates all share the topmost level.
    plan.gap_lanes[gap] = lanes.values().max().map_or(0, |deepest| deepest + 1);

    Ok(())
}

/// Runs entering one junction in this gap form one horizontal merge rail.
/// Each run is represented by the first member of its group, so all members'
/// constraints are checked together before a lane is assigned.
fn merge_lanes(topology: &Topology, plan: &Plan, gap: usize, runs: &[&Crossing]) -> Vec<usize> {
    runs.iter()
        .enumerate()
        .map(|(index, run)| {
            let destination = topology.connections[run.connection].destination;
            if matches!(destination, Destination::Junction(_))
                && plan.crossings[run.connection]
                    .last()
                    .is_some_and(|(last, _)| *last == gap)
            {
                (0..index)
                    .find(|&other| {
                        topology.connections[runs[other].connection].destination == destination
                    })
                    .unwrap_or(index)
            } else {
                index
            }
        })
        .collect()
}

/// Topological order of the lane requirements, lowest lane first, breaking ties
/// by run order so the result never depends on iteration order.
fn order(above: &[Vec<usize>], movable: &dyn Fn(usize) -> bool) -> Result<Vec<usize>, usize> {
    let mut placed = vec![false; above.len()];
    let mut ordered = Vec::with_capacity(above.len());
    while ordered.len() < above.len() {
        let next = (0..above.len())
            .find(|&run| !placed[run] && above[run].iter().all(|&earlier| placed[earlier]));
        let Some(next) = next else {
            // A cycle of lane requirements: the runs in it cannot all sit in
            // one gap, so one of them has to move. Blame one the search can
            // actually move.
            let unplaced = |run: &usize| !placed[*run];
            return Err((0..above.len())
                .filter(unplaced)
                .find(|&run| movable(run))
                .or_else(|| (0..above.len()).find(unplaced))
                .expect("an unfinished order leaves a run unplaced"));
        };
        placed[next] = true;
        ordered.push(next);
    }

    Ok(ordered)
}

/// Whether the search may move this connection. A select's distributor fans out
/// to its cases as one shared horizontal (RFC 0002 §4.5); sending one of those
/// runs down a column of its own would take the case row apart.
fn movable(topology: &Topology, connection: usize) -> bool {
    !matches!(
        topology.connections[connection].destination,
        Destination::Node(NodeId::Case { .. })
    )
}

/// Of a blocked pair, the connection the search should move: the one that was
/// in the way, unless it is a run that has to stay where it is.
fn mover(topology: &Topology, blocked: usize, other: usize) -> usize {
    if movable(topology, blocked) {
        blocked
    } else {
        other
    }
}

/// Two connections are drawn as one bundle when they leave one exit or reach
/// one destination, and may then share runs and meet where they join.
fn bundled(topology: &Topology, left: usize, right: usize) -> bool {
    let pair = (topology.connections[left], topology.connections[right]);
    pair.0.source == pair.1.source || pair.0.destination == pair.1.destination
}

/// The column one connection leaves its exit by. A select connection uses its
/// case's column. A later question branch joins its merge column immediately
/// when that column lies between its node and its own branch column, so the
/// side exit reaches the merge rail without a detour. RFC 0002 §8 puts the
/// merge on the first of the branches that continue, so a later branch turns
/// towards it; departing by a column to the right of its own is not a shortcut
/// but a crossing.
pub(super) fn departure_column(
    placement: &Placement,
    source: Source,
    destination: Destination,
) -> i32 {
    if let Some(case) = super::choice::case_destination(source, destination) {
        return placement.column(Vertex::Node(case));
    }
    if let (Source::Exit(exit), Destination::Junction(junction)) = (source, destination) {
        let merge = placement.column(Vertex::Junction(junction));
        if exit.branch.is_some_and(|branch| branch > 0)
            && placement.column(Vertex::Node(exit.node)) < merge
            && merge < placement.exit_column(exit)
        {
            return merge;
        }
    }

    match source {
        Source::Exit(exit) => placement.exit_column(exit),
        Source::Junction(junction) => placement.column(Vertex::Junction(junction)),
    }
}

/// One obstruction, reported at the block the blamed route leaves.
pub(super) fn obstruction(
    flow: &Flow,
    topology: &Topology,
    connection: usize,
    message: String,
) -> super::Obstruction {
    super::Obstruction {
        span: describe::vertex_span(
            flow,
            topology,
            Vertex::from(topology.connections[connection].source),
        ),
        message,
        loop_index: None,
        connection: Some(connection),
    }
}

/// Finds crossings that gap ordering alone cannot see, over the complete
/// polylines of an assembled arrangement.
pub(super) fn crossing(
    topology: &Topology,
    arrangement: &super::Arrangement,
) -> Option<(usize, usize)> {
    use super::verify::{Grid, polyline};
    let grid = Grid::of(topology, arrangement);
    let lines = (0..topology.connections.len())
        .map(|index| polyline(topology, arrangement, &grid, index))
        .collect::<Vec<_>>();
    super::verify::crossing(topology, &lines)
}
