//! Routes every connection as a plain orthogonal run and then checks the
//! result against RFC 0002 §8.
//!
//! A route descends in its exit's column, except that a question's side exit may
//! join a merge column immediately. It crosses each row gap it needs sideways in
//! one lane and enters its destination from above. Two connections may share a run
//! only when they leave one exit or reach one destination, which is what draws a
//! fan-out, a select's distributor and a wire merge as one bundle rather than as
//! routes hidden behind each other.
//!
//! The lanes of one row gap are not guessed. A horizontal run that passes over
//! another route's descent must lie below it, and one that passes over another
//! route's arrival must lie above it; those requirements form a partial order,
//! and its topological order is the lane order. A cycle means the gap has no
//! crossing-free arrangement, and the whole layout is then reported unroutable
//! rather than drawn.

use std::collections::{BTreeMap, BTreeSet};

use crate::topology::{Destination, NodeId, Source, Vertex};

use super::{COLUMN_WIDTH, Connection, Point, Rows, Scene, column_x, place::Placement};

/// What one connection does in one row gap: it enters from above at `enter`,
/// leaves downward at `exit`, and needs a lane when those differ.
#[derive(Clone, Copy)]
struct Crossing {
    connection: usize,
    enter: i32,
    exit: i32,
}

impl Crossing {
    const fn sideways(&self) -> bool {
        self.enter != self.exit
    }

    /// Whether a vertical at `x` would meet this run. The ends count: a run
    /// that stops on another route's descent touches it just as surely as one
    /// that passes over it.
    const fn spans(&self, x: i32) -> bool {
        let (left, right) = if self.enter < self.exit {
            (self.enter, self.exit)
        } else {
            (self.exit, self.enter)
        };
        x >= left && x <= right
    }
}

pub(super) struct Plan {
    /// Per connection, the gap it crosses and what it does there.
    crossings: Vec<Vec<(usize, Crossing)>>,
    /// Lane of each sideways run, keyed by connection and gap.
    lanes: BTreeMap<(usize, usize), usize>,
    gap_lanes: Vec<usize>,
}

impl Plan {
    pub(super) fn lanes_in(&self, gap: usize) -> usize {
        self.gap_lanes.get(gap).copied().unwrap_or(0)
    }
}

/// What to change when a row gap has no crossing-free arrangement. Both repairs
/// only ever add room, so a retry loop over them terminates.
pub(super) struct Blocked {
    /// The connection to move, and how far up the ladder of repairs it has to
    /// climb before it fits.
    pub(super) connection: usize,
    /// What could not be drawn, for the rendering error when nothing fits.
    pub(super) reason: String,
}

/// How a connection reaches its destination's column. The three are
/// alternatives, not stages: a repair replaces one with the next rather than
/// adding to it, so the shape that works is never overridden by a later one.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) enum Shape {
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

/// Plans the row gaps every connection crosses and the lane each sideways run
/// takes, from columns alone, so the gaps can then be sized to fit.
pub(super) fn plan(
    scene: &Scene,
    placement: &Placement,
    shapes: &BTreeMap<usize, Shape>,
) -> Result<Plan, Blocked> {
    // Where a descent may not pass. A junction counts: the routes that meet
    // there occupy its point just as a node occupies its box.
    let taken = scene
        .topology
        .vertices
        .iter()
        .map(|&vertex| (placement.row(vertex), placement.column(vertex)))
        .collect::<BTreeSet<_>>();

    let spans = scene
        .topology
        .connections
        .iter()
        .enumerate()
        .map(|(index, wire)| {
            let source = Vertex::from(wire.source);
            let destination = wire.destination;
            let shape = shapes.get(&index).copied().unwrap_or_default();
            // A descent through an intermediate row needs a column free of
            // nodes there. Try the arrival column, then the departure column,
            // before allocating a separate column for an actual obstacle.
            let top = placement.row(source);
            let bottom = placement.row(destination);
            let free_of_nodes =
                |column| (top + 1..bottom).all(|row| !taken.contains(&(row, column)));
            let departure = departure_column(placement, wire.source, wire.destination);
            let start = column_x(departure);
            let end = arrival_x(scene, placement, destination);
            let waypoint = match shape {
                Shape::Aside => None,
                // Producer branches keep separate descents until the merge rail.
                _ if matches!(wire.destination, Destination::Junction(_)) => {
                    free_of_nodes(departure).then_some(start)
                }
                Shape::Deferred => free_of_nodes(departure).then_some(start),
                Shape::Direct => free_of_nodes(placement.column(destination))
                    .then_some(end)
                    .or_else(|| free_of_nodes(departure).then_some(start)),
            };
            (top, bottom, start, end, waypoint)
        })
        .collect::<Vec<_>>();

    let columns = own_columns(scene, placement, shapes, &taken, &spans);

    let crossings = spans
        .iter()
        .enumerate()
        .map(|(index, &(top, bottom, start, end, waypoint))| {
            let waypoint = waypoint.unwrap_or_else(|| columns[&index]);
            (top..bottom)
                .map(|gap| {
                    let crossing = Crossing {
                        connection: index,
                        enter: if gap == top { start } else { waypoint },
                        exit: if gap + 1 == bottom { end } else { waypoint },
                    };
                    (gap, crossing)
                })
                .collect()
        })
        .collect();

    let mut plan = Plan {
        crossings,
        lanes: BTreeMap::new(),
        gap_lanes: vec![0; placement.rows],
    };
    assign_lanes(scene, &mut plan)?;

    Ok(plan)
}

/// A column of its own for every route that cannot descend in the column it
/// arrives at or departs from.
///
/// They are handed out from left to right by where their routes start, so two
/// of them stay side by side instead of nesting: a run that begins further
/// right also ends further right, which one lane order can always separate.
fn own_columns(
    scene: &Scene,
    placement: &Placement,
    shapes: &BTreeMap<usize, Shape>,
    taken: &BTreeSet<(usize, usize)>,
    spans: &[(usize, usize, i32, i32, Option<i32>)],
) -> BTreeMap<usize, i32> {
    let mut claimed: BTreeSet<(usize, usize)> = BTreeSet::new();
    let mut aside: BTreeSet<(usize, usize)> = BTreeSet::new();
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
        // A repair sent an Aside route out of another's way, and the nearest
        // free column is exactly where it was caught. It leaves the diagram on
        // the side it is already heading: a run that turned back across every
        // column would swallow the runs it was avoiding.
        let (lanes, free) = if shapes.get(&index).copied().unwrap_or_default() == Shape::Aside {
            (&mut aside, None)
        } else {
            (&mut claimed, Some(()))
        };
        let mut lane = if free.is_some() {
            placement
                .column(Vertex::from(scene.topology.connections[index].source))
                .min(placement.column(scene.topology.connections[index].destination))
        } else {
            0
        };
        while (top..=bottom).any(|row| {
            lanes.contains(&(row, lane)) || (free.is_some() && taken.contains(&(row, lane)))
        }) {
            lane += 1;
        }
        for row in top..=bottom {
            lanes.insert((row, lane));
        }
        columns.insert(
            index,
            match free {
                Some(()) => column_x(lane),
                None if end < start => column_x(0) - (lane as i32 + 1) * COLUMN_WIDTH,
                None => column_x(outside + lane),
            },
        );
    }

    columns
}

/// Orders the sideways runs of each row gap so that none meets a descent or an
/// arrival it is not allowed to meet.
fn assign_lanes(scene: &Scene, plan: &mut Plan) -> Result<(), Blocked> {
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
        let groups = merge_lanes(scene, plan, gap, &sideways);
        // Constraints belong to the group's first run. Other group members
        // occupy that same lane rather than drawing a second merge rail.
        let mut above: Vec<Vec<usize>> = vec![Vec::new(); sideways.len()];
        for (a, run) in sideways.iter().enumerate() {
            let a = groups[a];
            for other in &crossings {
                if other.connection == run.connection
                    || bundled(scene, run.connection, other.connection)
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
                            connection: mover(scene, other.connection, run.connection),
                            reason: format!(
                                "{} runs across {}, which descends through the same row gap",
                                name(scene, run.connection),
                                name(scene, other.connection)
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
                        connection: mover(scene, other.connection, run.connection),
                        reason: format!(
                            "{} runs across both ends of {} in one row gap",
                            name(scene, run.connection),
                            name(scene, other.connection)
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

        let ordered =
            order(&above, &|run| movable(scene, sideways[run].connection)).map_err(|run| {
                Blocked {
                    connection: sideways[run].connection,
                    reason: format!(
                        "no lane order draws one row gap: {} is caught in a cycle of them",
                        name(scene, sideways[run].connection)
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
        for (run, group) in sideways.iter().zip(groups) {
            plan.lanes.insert((run.connection, gap), lanes[&group]);
        }
        // The gap needs room for the levels actually occupied, not for one per
        // group: runs no ordering separates all share the topmost level.
        plan.gap_lanes[gap] = lanes.values().max().map_or(0, |deepest| deepest + 1);
    }

    Ok(())
}

/// Runs entering one junction in this gap form one horizontal merge rail.
/// Each run is represented by the first member of its group, so all members'
/// constraints are checked together before a lane is assigned.
fn merge_lanes(scene: &Scene, plan: &Plan, gap: usize, runs: &[&Crossing]) -> Vec<usize> {
    runs.iter()
        .enumerate()
        .map(|(index, run)| {
            let destination = scene.topology.connections[run.connection].destination;
            if matches!(destination, Destination::Junction(_))
                && plan.crossings[run.connection]
                    .last()
                    .is_some_and(|(last, _)| *last == gap)
            {
                (0..index)
                    .find(|&other| {
                        scene.topology.connections[runs[other].connection].destination
                            == destination
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
            // one gap, so one of them has to move. Blame one a repair can
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

/// Whether a repair may move this connection. A select's distributor fans out
/// to its cases as one shared horizontal (RFC 0002 §4.4); sending one of those
/// runs down a column of its own would take the case row apart.
fn movable(scene: &Scene, connection: usize) -> bool {
    !matches!(
        scene.topology.connections[connection].destination,
        Destination::Node(NodeId::Case { .. })
    )
}

/// Of a blocked pair, the connection a repair should move: the one that was in
/// the way, unless it is a run that has to stay where it is.
fn mover(scene: &Scene, blocked: usize, other: usize) -> usize {
    if movable(scene, blocked) {
        blocked
    } else {
        other
    }
}

/// Two connections are drawn as one bundle when they leave one exit or reach
/// one destination, and may then share runs and meet where they join.
fn bundled(scene: &Scene, left: usize, right: usize) -> bool {
    let pair = (
        scene.topology.connections[left],
        scene.topology.connections[right],
    );
    pair.0.source == pair.1.source || pair.0.destination == pair.1.destination
}

fn departure_column(placement: &Placement, source: Source, destination: Destination) -> usize {
    if let (Source::Exit(exit), Destination::Junction(junction)) = (source, destination) {
        let merge = placement.column(Vertex::Junction(junction));
        if exit.branch.is_some_and(|branch| branch > 0)
            && placement.column(Vertex::Node(exit.node)) < merge
        {
            return merge;
        }
    }

    match source {
        Source::Exit(exit) => placement.exit_column(exit),
        Source::Junction(junction) => placement.column(Vertex::Junction(junction)),
    }
}

fn arrival_x(scene: &Scene, placement: &Placement, destination: Vertex) -> i32 {
    match destination {
        Vertex::Node(node) => scene.node(node).x,
        Vertex::Junction(junction) => column_x(placement.column(Vertex::Junction(junction))),
    }
}

pub(super) fn emit(
    scene: &Scene,
    placement: &Placement,
    plan: &Plan,
    rows: &Rows,
) -> Vec<Connection> {
    scene
        .topology
        .connections
        .iter()
        .enumerate()
        .map(|(index, wire)| {
            let start = match wire.source {
                Source::Exit(exit) => scene.exit_anchor(exit),
                Source::Junction(junction) => {
                    junction_point(scene, placement, plan, rows, junction)
                }
            };
            let end = match wire.destination {
                Vertex::Node(node) => scene.top_anchor(node),
                Vertex::Junction(junction) => {
                    junction_point(scene, placement, plan, rows, junction)
                }
            };

            // A side exit reaches its branch column horizontally before it
            // descends into the row gaps. The brancher's footprint reserves
            // this space beside the node.
            let mut points = vec![
                start,
                Point {
                    x: column_x(departure_column(placement, wire.source, wire.destination)),
                    y: start.y,
                },
            ];
            for (gap, crossing) in &plan.crossings[index] {
                if !crossing.sideways() {
                    continue;
                }
                let y = rows.lane_y(*gap, plan.lanes[&(index, *gap)], plan.lanes_in(*gap));
                points.push(Point {
                    x: crossing.enter,
                    y,
                });
                points.push(Point {
                    x: crossing.exit,
                    y,
                });
            }
            points.push(end);

            Connection {
                source: wire.source,
                destination: wire.destination,
                points: straighten(points),
            }
        })
        .collect()
}

/// A junction is a point, not a node: the routes that meet there arrive at it
/// and the common segment leaves from it. Use the merge rail's actual lane,
/// so a side route ends horizontally instead of turning down over that segment.
fn junction_point(
    scene: &Scene,
    placement: &Placement,
    plan: &Plan,
    rows: &Rows,
    junction: usize,
) -> Point {
    let row = placement.row(Vertex::Junction(junction));
    let gap = row - 1;
    let lane = scene
        .topology
        .connections
        .iter()
        .enumerate()
        .filter(|(_, wire)| wire.destination == Destination::Junction(junction))
        .filter_map(|(connection, _)| plan.lanes.get(&(connection, gap)))
        .max();
    Point {
        x: column_x(placement.column(Vertex::Junction(junction))),
        y: lane.map_or_else(
            || rows.junction_y(row),
            |&lane| rows.lane_y(gap, lane, plan.lanes_in(gap)),
        ),
    }
}

/// Drops repeated points and collapses runs of collinear ones, so every bend in
/// an emitted route is a real right angle.
fn straighten(points: Vec<Point>) -> Vec<Point> {
    let mut straight: Vec<Point> = Vec::with_capacity(points.len());
    for point in points {
        if straight.last() == Some(&point) {
            continue;
        }
        while straight.len() >= 2 {
            let previous = straight[straight.len() - 2];
            let last = straight[straight.len() - 1];
            let collinear = (previous.x == last.x && last.x == point.x)
                || (previous.y == last.y && last.y == point.y);
            if collinear {
                straight.pop();
            } else {
                break;
            }
        }
        straight.push(point);
    }
    straight
}

/// Reports the first RFC 0002 §8 rule the emitted routes break, if any.
pub(super) fn verify(scene: &Scene) -> Option<String> {
    for connection in &scene.connections {
        if connection.points.len() < 2 {
            return Some("a connection has no route".to_owned());
        }
        for segment in connection.points.windows(2) {
            let (a, b) = (segment[0], segment[1]);
            if a.x != b.x && a.y != b.y {
                return Some("a connection bends other than at a right angle".to_owned());
            }
            if b.y < a.y {
                return Some("a connection moves upward".to_owned());
            }
            if scene
                .nodes
                .iter()
                .filter(|node| !touches(connection, node.id))
                .any(|node| enters(a, b, Scene::bounds(node)))
            {
                return Some("a connection passes through a node".to_owned());
            }
        }
        if overlaps_itself(&connection.points) {
            return Some("a connection overlaps itself".to_owned());
        }
        if matches!(connection.destination, Destination::Junction(_))
            && connection.points.windows(2).any(|segment| {
                segment[0].x != segment[1].x && segment[0].y > connection.points[0].y
            })
            && connection.points[connection.points.len() - 2].y
                != connection.points[connection.points.len() - 1].y
        {
            return Some("a merge side route turns downward before its endpoint".to_owned());
        }
    }

    for (index, connection) in scene.connections.iter().enumerate() {
        for other in &scene.connections[index + 1..] {
            let shared =
                connection.source == other.source || connection.destination == other.destination;
            let meetings = if shared {
                bundle_meetings(&connection.points, &other.points)
            } else {
                common_junction(connection, other).into_iter().collect()
            };
            for left in connection.points.windows(2) {
                for right in other.points.windows(2) {
                    let parallel = (left[0].x == left[1].x) == (right[0].x == right[1].x);
                    let allowed = if parallel {
                        shared
                    } else {
                        meetings
                            .iter()
                            .any(|&point| on_segment(point, left) && on_segment(point, right))
                    };
                    if !allowed && crosses(left[0], left[1], right[0], right[1]) {
                        return Some("two connections cross".to_owned());
                    }
                }
            }
        }
    }

    None
}

/// A bundle may split or join at a common endpoint or the end of a shared run.
/// Sharing an exit or destination does not permit crossings elsewhere.
fn bundle_meetings(left: &[Point], right: &[Point]) -> Vec<Point> {
    let mut meetings = Vec::new();
    for (a, b) in [(left.first(), right.first()), (left.last(), right.last())] {
        if a == b {
            meetings.extend(a.copied());
        }
    }
    for a in left.windows(2) {
        for b in right.windows(2) {
            if (a[0].x == a[1].x) == (b[0].x == b[1].x) && crosses(a[0], a[1], b[0], b[1]) {
                meetings.extend(
                    a.iter()
                        .chain(b)
                        .copied()
                        .filter(|&point| on_segment(point, a) && on_segment(point, b)),
                );
            }
        }
    }
    meetings
}

fn on_segment(point: Point, segment: &[Point]) -> bool {
    point.x >= segment[0].x.min(segment[1].x)
        && point.x <= segment[0].x.max(segment[1].x)
        && point.y >= segment[0].y.min(segment[1].y)
        && point.y <= segment[0].y.max(segment[1].y)
}

/// Only an incoming route and an outgoing route of the same junction may meet
/// perpendicularly at their endpoints. Sharing any length still counts as overlap.
fn common_junction(left: &Connection, right: &Connection) -> Option<Point> {
    [(left, right), (right, left)]
        .into_iter()
        .find_map(|(incoming, outgoing)| {
            if let Destination::Junction(junction) = incoming.destination
                && outgoing.source == Source::Junction(junction)
            {
                incoming
                    .points
                    .last()
                    .copied()
                    .filter(|point| outgoing.points.first() == Some(point))
            } else {
                None
            }
        })
}

fn touches(connection: &Connection, node: NodeId) -> bool {
    matches!(connection.source, Source::Exit(exit) if exit.node == node)
        || connection.destination == Destination::Node(node)
}

fn overlaps_itself(points: &[Point]) -> bool {
    points.windows(3).any(|points| {
        (points[0].x == points[1].x) == (points[1].x == points[2].x)
            && crosses(points[0], points[1], points[1], points[2])
    }) || points.windows(2).enumerate().any(|(index, segment)| {
        points[index + 2..]
            .windows(2)
            .any(|other| crosses(segment[0], segment[1], other[0], other[1]))
    })
}

/// Whether a segment passes through a rectangle rather than merely touching it.
fn enters(a: Point, b: Point, bounds: (i32, i32, i32, i32)) -> bool {
    let (left, top, right, bottom) = bounds;
    if a.x == b.x {
        a.x > left && a.x < right && a.y.min(b.y) < bottom && a.y.max(b.y) > top
    } else {
        a.y > top && a.y < bottom && a.x.min(b.x) < right && a.x.max(b.x) > left
    }
}

/// Whether two orthogonal segments meet: parallel ones only where they overlap,
/// perpendicular ones wherever they touch.
fn crosses(a: Point, b: Point, c: Point, d: Point) -> bool {
    let horizontal = a.y == b.y;
    if horizontal == (c.y == d.y) {
        if horizontal {
            a.y == c.y && a.x.min(b.x).max(c.x.min(d.x)) < a.x.max(b.x).min(c.x.max(d.x))
        } else {
            a.x == c.x && a.y.min(b.y).max(c.y.min(d.y)) < a.y.max(b.y).min(c.y.max(d.y))
        }
    } else {
        let (across, down) = if horizontal {
            ((a, b), (c, d))
        } else {
            ((c, d), (a, b))
        };
        down.0.x >= across.0.x.min(across.1.x)
            && down.0.x <= across.0.x.max(across.1.x)
            && across.0.y >= down.0.y.min(down.1.y)
            && across.0.y <= down.0.y.max(down.1.y)
    }
}

/// Names one connection by the ends it joins, for a diagnostic.
fn name(scene: &Scene, connection: usize) -> String {
    let wire = scene.topology.connections[connection];
    let end = |vertex: Vertex| match vertex {
        Vertex::Node(node) => {
            let node = scene.topology.node(node);
            let label = node.label.trim();
            format!("{:?}({label})", node.kind)
        }
        Vertex::Junction(junction) => {
            format!(
                "merge({})",
                scene.topology.junctions[junction].wires.join("+")
            )
        }
    };
    let branch = match wire.source {
        Source::Exit(crate::topology::ExitId {
            branch: Some(branch),
            ..
        }) => format!(" branch {}", branch + 1),
        _ => String::new(),
    };

    format!(
        "[{end_from}{branch} -> {end_to}]",
        end_from = end(Vertex::from(wire.source)),
        end_to = end(wire.destination)
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::topology::{ExitId, Topology};

    fn routes(paths: &[&[(i32, i32)]]) -> Scene {
        let connections = paths
            .iter()
            .enumerate()
            .map(|(index, points)| Connection {
                source: Source::Exit(ExitId::of(NodeId::Block(0))),
                destination: Destination::Node(NodeId::Block(index + 2)),
                points: points.iter().map(|&(x, y)| Point { x, y }).collect(),
            })
            .collect();
        Scene {
            width: 10,
            height: 10,
            topology: Topology {
                nodes: vec![],
                exits: vec![],
                junctions: vec![],
                connections: vec![],
                vertices: vec![],
            },
            nodes: vec![],
            connections,
            labels: vec![],
        }
    }

    #[test]
    fn bundled_routes_may_share_a_rail_but_must_not_cross() {
        let rail: &[&[(i32, i32)]] = &[
            &[(0, 0), (0, 2), (4, 2), (4, 8)],
            &[(0, 0), (0, 2), (8, 2), (8, 8)],
        ];
        let crossing: &[&[(i32, i32)]] = &[
            &[(0, 0), (0, 2), (6, 2), (6, 8)],
            &[(0, 0), (4, 0), (4, 4), (8, 4), (8, 8)],
        ];
        for (paths, expected) in [(rail, None), (crossing, Some("two connections cross"))] {
            let mut scene = routes(paths);
            assert_eq!(verify(&scene).as_deref(), expected);
            // Reversing time turns a distributor into a merge with the same geometry.
            for (index, route) in scene.connections.iter_mut().enumerate() {
                route.source = Source::Exit(ExitId::of(NodeId::Block(index)));
                route.destination = Destination::Node(NodeId::Block(2));
                route.points.reverse();
                for point in &mut route.points {
                    point.y = 8 - point.y;
                }
            }
            assert_eq!(verify(&scene).as_deref(), expected);
        }
    }

    #[test]
    fn invalid_routes_are_rejected_before_serialization() {
        for (path, reason) in [
            (vec![(0, 0)], "a connection has no route"),
            (
                vec![(0, 0), (4, 4)],
                "a connection bends other than at a right angle",
            ),
            (vec![(0, 4), (0, 0)], "a connection moves upward"),
            (
                vec![(0, 0), (0, 2), (4, 2), (2, 2)],
                "a connection overlaps itself",
            ),
        ] {
            assert_eq!(verify(&routes(&[&path])).as_deref(), Some(reason));
        }
        let mut scene = routes(&[&[(0, 0), (0, 8)]]);
        scene.nodes.push(super::super::Node {
            id: NodeId::Block(1),
            x: 0,
            y: 4,
            width: 2,
            height: 2,
            lines: vec![],
        });
        assert_eq!(
            verify(&scene).as_deref(),
            Some("a connection passes through a node")
        );

        let mut scene = routes(&[&[(0, 0), (0, 8)], &[(0, 0), (0, 4), (4, 4), (4, 8)]]);
        assert_eq!(verify(&scene), None);
        scene.connections[1].source = Source::Exit(ExitId::of(NodeId::Block(1)));
        assert_eq!(verify(&scene).as_deref(), Some("two connections cross"));
    }

    #[test]
    fn a_branch_may_turn_into_the_merge_column_at_its_source() {
        let mut scene = routes(&[&[(0, 0), (4, 0), (4, 8)]]);
        scene.connections[0].destination = Destination::Junction(0);
        assert_eq!(verify(&scene), None);

        scene.connections[0].points = [(0, 0), (0, 4), (4, 4), (4, 8)]
            .into_iter()
            .map(|(x, y)| Point { x, y })
            .collect();
        assert_eq!(
            verify(&scene).as_deref(),
            Some("a merge side route turns downward before its endpoint")
        );
    }
}
