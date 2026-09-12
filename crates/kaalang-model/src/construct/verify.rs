//! Checks a arrangement against RFC 0002, independently of the search that
//! produced it.
//!
//! The check reads only the arrangement and the topology. It rebuilds every
//! route as an orthogonal polyline on the abstract grid the arrangement
//! describes — one line per rank, one per lane of each rank gap — and holds the
//! result to the same rules a renderer's final geometry must meet. A positive
//! result is a witness that the arrangement conforms; it says nothing about
//! whether another arrangement exists.

use crate::geometry::{
    Point, bundle_meetings, compatible, on_segment, overlaps_itself, straighten, turns_downward,
};
use crate::model::Flow;
use crate::topology::{Destination, NodeId, Source, Topology, Vertex};

use super::{Arrangement, Side};

/// The abstract grid a arrangement describes. Every rank gets a line of its
/// own and every lane of the gap below it one more; every column gets a
/// position of its own with room beside it for the return contours that climb
/// between columns.
///
/// The lanes of one gap are packed against the rank below them, exactly as a
/// presentation packs them against the following node row. A rank carrying a
/// node keeps one line clear for the captures drawn above it, and a rank
/// carrying only junctions does not — so on such a rank the deepest lane of the
/// gap above coincides with the rank's own line. That coincidence is real in
/// pixels, so the check has to see it too.
pub(super) struct Grid {
    step: i32,
    /// How many lanes each rank gap uses.
    lanes: Vec<usize>,
    /// Whether each rank carries a node, and so reserves a line for captures.
    nodes: Vec<bool>,
    /// How far apart two columns sit, so that every contour lane a topology can
    /// need fits between them.
    scale: i32,
}

/// How many contour lanes one side of a column offers.
///
/// Only a loop return climbs in the space beside a column, and every loop has
/// one return, so a topology never needs more lanes there than it has loops.
/// This is a fact about the topology, not about any presentation's spacing: a
/// renderer holds whatever this many lanes require.
pub(super) fn contour_lanes(topology: &Topology) -> usize {
    topology.loops.len()
}

impl Grid {
    pub(super) fn of(topology: &Topology, arrangement: &Arrangement) -> Self {
        let deepest = arrangement.gap_lanes.iter().copied().max().unwrap_or(0);
        let mut nodes = vec![false; arrangement.ranks + 1];
        for node in &topology.nodes {
            if let Some(&rank) = arrangement.rank.get(&Vertex::Node(node.id))
                && let Some(carries) = nodes.get_mut(rank)
            {
                *carries = true;
            }
        }
        Self {
            step: 2 + i32::try_from(deepest).unwrap_or(i32::MAX - 2),
            lanes: arrangement.gap_lanes.clone(),
            nodes,
            // Every lane of both sides of a column gets a position of its own,
            // plus the column's own, so the deepest lane beside one column
            // never reaches the shallowest lane beside the next.
            scale: 2 * i32::try_from(contour_lanes(topology)).unwrap_or(i32::MAX / 4) + 1,
        }
    }

    /// The position of one column.
    pub(super) const fn column(&self, column: i32) -> i32 {
        column * self.scale
    }

    /// The position of one contour lane beside a column. Lane 0 sits
    /// immediately outside the column; later lanes step further out.
    pub(super) const fn contour(&self, contour: super::Contour) -> i32 {
        let offset = contour.lane as i32 + 1;
        match contour.side {
            Side::Left => self.column(contour.column) - offset,
            Side::Right => self.column(contour.column) + offset,
        }
    }

    /// The line a rank's vertices sit on.
    pub(super) const fn rank(&self, rank: usize) -> i32 {
        rank as i32 * self.step
    }

    /// The line one lane of one rank gap occupies, packed against the rank
    /// below it.
    ///
    /// A gap outside the arrangement counts as holding this lane alone. Only
    /// `coverage` rejects such a gap, and it runs first, so this decides
    /// nothing for an arrangement the search produces; it keeps the check
    /// reporting rather than panicking on one it never would.
    pub(super) fn lane(&self, gap: usize, lane: usize) -> i32 {
        let capture = i32::from(self.nodes.get(gap + 1).copied().unwrap_or(false));
        let lanes = self.lanes.get(gap).copied().unwrap_or(lane + 1);
        self.rank(gap + 1) - capture - (lanes.saturating_sub(lane + 1)) as i32
    }
}

/// The line a junction's routes meet on: the deepest lane any of them takes in
/// the gap above it, so a side route finishes horizontally on the rail rather
/// than turning down over the continuation below it (RFC 0002 §8).
pub(super) fn junction_line(
    topology: &Topology,
    arrangement: &Arrangement,
    grid: &Grid,
    junction: usize,
) -> i32 {
    let rank = arrangement.rank[&Vertex::Junction(junction)];
    // Every junction has a producer above it, so it never sits on the first
    // rank; a projection that put one there has no rail to meet on.
    let Some(gap) = rank.checked_sub(1) else {
        return grid.rank(rank);
    };
    arrangement
        .deepest_lane(topology, junction, gap)
        .map_or_else(|| grid.rank(rank), |lane| grid.lane(gap, lane))
}

/// The line one endpoint of a route sits on.
fn endpoint_line(
    topology: &Topology,
    arrangement: &Arrangement,
    grid: &Grid,
    vertex: Vertex,
) -> i32 {
    match vertex {
        Vertex::Node(_) => grid.rank(arrangement.rank[&vertex]),
        Vertex::Junction(junction) => junction_line(topology, arrangement, grid, junction),
    }
}

/// One connection's route as an orthogonal polyline, in drawing order.
pub(super) fn polyline(
    topology: &Topology,
    arrangement: &Arrangement,
    grid: &Grid,
    index: usize,
) -> Vec<Point> {
    let wire = topology.connections[index];
    let route = &arrangement.routes[index];
    // A route leaves its node's own boundary, whatever branch column it then
    // descends in: the exit is on the node, not on the column beside it.
    let departure = arrangement.column[&Vertex::from(wire.source)];
    let source_line = endpoint_line(topology, arrangement, grid, Vertex::from(wire.source));
    let mut points = vec![
        Point {
            x: grid.column(departure),
            y: source_line,
        },
        Point {
            x: grid.column(route.departure),
            y: source_line,
        },
    ];
    for run in &route.runs {
        let line = grid.lane(run.gap, run.lane);
        points.push(Point {
            x: grid.column(run.enter),
            y: line,
        });
        points.push(Point {
            x: grid.column(run.exit),
            y: line,
        });
    }
    points.push(Point {
        x: grid.column(route.arrival),
        y: endpoint_line(topology, arrangement, grid, wire.destination),
    });
    straighten(points)
}

/// One loop return as an orthogonal polyy: out of the tail, up the contour,
/// and horizontally into the entry (RFC 0002 §8).
pub(super) fn return_polyline(
    topology: &Topology,
    arrangement: &Arrangement,
    grid: &Grid,
    tail: usize,
    entry: usize,
    contour: super::Contour,
) -> Vec<Point> {
    let tail_line = junction_line(topology, arrangement, grid, tail);
    let entry_line = junction_line(topology, arrangement, grid, entry);
    let aside = grid.contour(contour);
    straighten(vec![
        Point {
            x: grid.column(arrangement.column[&Vertex::Junction(tail)]),
            y: tail_line,
        },
        Point {
            x: aside,
            y: tail_line,
        },
        Point {
            x: aside,
            y: entry_line,
        },
        Point {
            x: grid.column(arrangement.column[&Vertex::Junction(entry)]),
            y: entry_line,
        },
    ])
}

/// Where two routes are allowed to meet, and whether they may share a run.
///
/// RFC 0002 §8 lets connections leaving one exit or reaching one destination
/// share a collinear segment and split or join where they touch. Otherwise only
/// the incoming and outgoing routes of one junction may meet, and only at that
/// junction's own point.
pub(super) fn meetings(
    left: (Source, Destination),
    left_line: &[Point],
    right: (Source, Destination),
    right_line: &[Point],
) -> (bool, Vec<Point>) {
    let shared = left.0 == right.0 || left.1 == right.1;
    let mut points = if shared {
        bundle_meetings(left_line, right_line)
    } else {
        Vec::new()
    };
    // A vertex is one point here and a box or a junction in a drawing, so two
    // routes that both touch it meet there rather than crossing: they attach to
    // different parts of its boundary.
    let ends = |pair: (Source, Destination), line: &[Point]| {
        [
            (Vertex::from(pair.0), line.first().copied()),
            (pair.1, line.last().copied()),
        ]
    };
    for (vertex, point) in ends(left, left_line) {
        for (other, _) in ends(right, right_line) {
            if vertex == other {
                points.extend(point);
            }
        }
    }
    (shared, points)
}

/// The pair of ends one connection joins.
pub(super) fn ends(topology: &Topology, index: usize) -> (Source, Destination) {
    let wire = topology.connections[index];
    (wire.source, wire.destination)
}

/// Reports the first rule a arrangement breaks, if any.
///
/// # Errors
///
/// Returns the rule and the items that break it.
pub(crate) fn arrangement(
    flow: &Flow,
    topology: &Topology,
    arrangement: &Arrangement,
) -> Result<(), String> {
    coverage(topology, arrangement)?;
    super::end::verify(topology, arrangement)?;
    order(topology, arrangement)?;
    branch_columns(flow, topology, arrangement)?;
    let grid = Grid::of(topology, arrangement);
    let lines = (0..topology.connections.len())
        .map(|index| polyline(topology, arrangement, &grid, index))
        .collect::<Vec<_>>();
    routes(topology, arrangement, &grid, &lines)?;
    returns(flow, topology, arrangement, &grid, &lines)
}

/// Every vertex has a rank and a column, every connection a corridor, and
/// every loop a contour.
fn coverage(topology: &Topology, arrangement: &Arrangement) -> Result<(), String> {
    for &vertex in &topology.vertices {
        if !arrangement.rank.contains_key(&vertex) {
            return Err(format!("{vertex:?} has no rank"));
        }
        if !arrangement.column.contains_key(&vertex) {
            return Err(format!("{vertex:?} has no column"));
        }
    }
    for exit in &topology.exits {
        if !arrangement.exit_offset.contains_key(&exit.id) {
            return Err(format!("{:?} has no branch column", exit.id));
        }
    }
    if arrangement.routes.len() != topology.connections.len() {
        return Err("the arrangement covers a different number of connections".to_owned());
    }
    if arrangement.contours.len() != topology.loops.len() {
        return Err("the arrangement covers a different number of loop returns".to_owned());
    }
    if arrangement.gap_lanes.len() != arrangement.ranks {
        return Err("the arrangement counts lanes for a different number of rank gaps".to_owned());
    }
    for (index, route) in arrangement.routes.iter().enumerate() {
        for run in &route.runs {
            let lanes = arrangement.gap_lanes.get(run.gap).copied().unwrap_or(0);
            if run.lane >= lanes {
                return Err(format!(
                    "connection {} takes lane {} of a rank gap with {lanes}",
                    index + 1,
                    run.lane
                ));
            }
        }
    }
    // A corridor has to end at the vertices its connection joins, and leave by
    // a column that exit owns: either its own branch column, or the column of
    // the merge a later question branch joins at once (RFC 0002 §8).
    for (index, wire) in topology.connections.iter().enumerate() {
        let route = &arrangement.routes[index];
        if route.arrival != arrangement.column[&wire.destination] {
            return Err(format!(
                "connection {} arrives away from its destination",
                index + 1
            ));
        }
        let owned = match wire.source {
            Source::Exit(exit) => {
                let own =
                    arrangement.column[&Vertex::Node(exit.node)] + arrangement.exit_offset[&exit];
                // A later question branch may join its merge column at once,
                // which means turning towards it: the merge is left of the
                // branch's own column, never right of it.
                let merge = arrangement.column[&wire.destination];
                let shortcut = matches!(wire.destination, Destination::Junction(_))
                    && exit.branch.is_some_and(|branch| branch > 0)
                    && merge < own
                    && route.departure == merge;
                route.departure == own || shortcut
            }
            Source::Junction(junction) => {
                route.departure == arrangement.column[&Vertex::Junction(junction)]
            }
        };
        if !owned {
            return Err(format!(
                "connection {} leaves by a column its exit does not own",
                index + 1
            ));
        }
    }
    Ok(())
}

/// Forward connections descend, and so does placement-only precedence.
fn order(topology: &Topology, arrangement: &Arrangement) -> Result<(), String> {
    for (relation, edges) in [
        ("a connection", &topology.connections),
        ("placement precedence", &topology.order),
    ] {
        for edge in edges {
            let from = arrangement.rank[&Vertex::from(edge.source)];
            let to = arrangement.rank[&edge.destination];
            if from >= to {
                return Err(format!("{relation} does not descend: {from} to {to}"));
            }
        }
    }
    Ok(())
}

/// A selection's branches leave it left to right in authored order, its shared
/// continuations sit in the column their group's first branch reached, and
/// each convergence group keeps the columns it reserves (RFC 0002 §8).
fn branch_columns(
    flow: &Flow,
    topology: &Topology,
    arrangement: &Arrangement,
) -> Result<(), String> {
    let reachable = super::regions::reachable(topology);
    for block in super::regions::branchers(flow) {
        let regions = super::regions::regions(flow, topology, &reachable, block);
        let count = flow.blocks[block].branch_count();
        let starts = (0..count)
            .map(|branch| {
                super::regions::branch_column(arrangement, flow, block, branch).ok_or_else(|| {
                    format!(
                        "block {} has no column for branch {}",
                        block + 1,
                        branch + 1
                    )
                })
            })
            .collect::<Result<Vec<_>, _>>()?;
        // RFC 0002 §8: the first answer or case continues the current column
        // and the rest appear to its right, in authored order. A choice lives
        // in its case nodes' columns, a question in its exits' branch columns.
        if !starts.windows(2).all(|pair| pair[0] < pair[1]) {
            return Err(format!(
                "block {} draws its branches out of authored order: {starts:?}",
                block + 1
            ));
        }
        let own = arrangement.column[&Vertex::Node(NodeId::Block(block))];
        if starts.first() != Some(&own) {
            return Err(format!(
                "block {} does not continue its own column into its first branch",
                block + 1
            ));
        }
        for (entry, first) in super::regions::continuations(topology, block, &regions.branches) {
            // Read the first branch's actual approach, which may have moved
            // through a nested selection since leaving this brancher.
            let mut frontier = vec![entry];
            let mut approaches = Vec::new();
            while let Some(vertex) = frontier.pop() {
                for connection in topology.incoming(vertex) {
                    match connection.source {
                        Source::Junction(junction) => frontier.push(Vertex::Junction(junction)),
                        Source::Exit(exit)
                            if regions.branches[first].contains(&Vertex::Node(exit.node))
                                || (exit.node == NodeId::Block(block)
                                    && exit.branch == Some(first)) =>
                        {
                            approaches.push(
                                arrangement.column[&Vertex::Node(exit.node)]
                                    + arrangement.exit_offset[&exit],
                            );
                        }
                        Source::Exit(_) => {}
                    }
                }
            }
            let column = approaches.into_iter().min().ok_or_else(|| {
                format!("continuation entry {entry:?} has no first-branch approach")
            })?;
            if arrangement.column[&entry] != column {
                return Err(format!(
                    "block {} draws continuation entry {entry:?} away from its first branch's approach column {column}",
                    block + 1,
                ));
            }
        }
        distributor_order(topology, arrangement, block)?;
        reserved_columns(arrangement, block, &regions)?;
    }
    Ok(())
}

/// The routes leaving one exit reach their destinations in authored order.
///
/// A selection's branches are arranged left to right in authored order
/// (RFC 0002 §8), and the routes that carry them are too. They may share the
/// collinear run their common exit gives them, so nothing crosses if they
/// descend in another order — but then the branch written first would be
/// drawn beyond the one written after it, and the diagram would no longer
/// show the authored order it claims to.
///
/// It holds only where one exit carries several branches — a choice's
/// distributor. Several connections of a question's branch exit carry one
/// branch to several consumers, and nothing orders those among themselves.
///
/// On its own this adds little: a route that ends in the column it descends
/// in is already covered by the branch columns above. What it catches is a
/// route that descends elsewhere and turns in, which the corridor shapes allow
/// and no other rule here would see.
fn distributor_order(
    topology: &Topology,
    arrangement: &Arrangement,
    block: usize,
) -> Result<(), String> {
    for exit in topology
        .exits
        .iter()
        .filter(|exit| exit.id.node == NodeId::Block(block))
        .filter(|exit| super::regions::carries_branches(topology, exit.id))
    {
        let descents = topology
            .leaving(exit.id)
            .map(|wire| {
                let index = topology
                    .connections
                    .iter()
                    .position(|other| other == wire)
                    .expect("a leaving connection is projected");
                let route = &arrangement.routes[index];
                route.runs.first().map_or(route.departure, |run| run.exit)
            })
            .collect::<Vec<_>>();
        if !descents.windows(2).all(|pair| pair[0] < pair[1]) {
            return Err(format!(
                "block {} sends the routes of {:?} down in the order {descents:?}, not the authored one",
                block + 1,
                exit.id
            ));
        }
    }
    Ok(())
}

/// A convergence group reserves the columns of everything its branches draw,
/// and a sibling outside the group stays clear of that whole area on the side
/// its authored position puts it (RFC 0002 §8).
///
/// The group and its area come from the topology — which branches meet, and
/// what they draw — so this holds an arrangement to a range it did not choose.
/// Reading a width the same search recorded would compare a value with itself.
///
/// Only what the sibling alone draws is held to its side. A vertex two
/// branches reach is a convergence, and belongs to the area of the group that
/// meets there.
///
/// A branch the group encloses — its authored position falls between two
/// members — stays inside the band instead: leaving it would put part of the
/// branch past one written after it, which authored branch order forbids.
/// Branches that never meet are held to nothing: RFC 0002 §8 reserves columns
/// for a convergence group, not for every branch, so their subtrees may
/// interleave.
fn reserved_columns(
    arrangement: &Arrangement,
    block: usize,
    regions: &super::regions::Regions,
) -> Result<(), String> {
    /// Where a sibling of a convergence group sits relative to its members.
    enum Sits {
        Before,
        Inside,
        After,
    }

    let column = |vertex: &Vertex| arrangement.column[vertex];
    for group in &regions.groups {
        let Some(low) = group.area.iter().map(column).min() else {
            continue;
        };
        let high = group.area.iter().map(column).max().unwrap_or(low);
        let first = *group.members.first().expect("a group has members");
        let last = *group.members.last().expect("a group has members");
        for branch in 0..regions.branches.len() {
            if group.members.contains(&branch) {
                continue;
            }
            let sits = if branch < first {
                Sits::Before
            } else if branch > last {
                Sits::After
            } else {
                Sits::Inside
            };
            for vertex in &regions.outside(group, branch) {
                let at = column(vertex);
                let kept = match sits {
                    Sits::Before => at < low,
                    Sits::After => at > high,
                    Sits::Inside => low < at && at < high,
                };
                if kept {
                    continue;
                }
                return Err(format!(
                    "block {} draws {vertex:?} of branch {} in column {at}, {} the columns {low} to {high} its convergence group {:?} reserves",
                    block + 1,
                    branch + 1,
                    match sits {
                        Sits::Inside => "outside",
                        _ => "inside",
                    },
                    group.members
                ));
            }
        }
    }
    Ok(())
}

/// Every vertex as a point of the grid.
fn vertex_points(
    topology: &Topology,
    arrangement: &Arrangement,
    grid: &Grid,
) -> Vec<(Point, Vertex)> {
    topology
        .vertices
        .iter()
        .map(|&vertex| {
            (
                Point {
                    x: grid.column(arrangement.column[&vertex]),
                    y: grid.rank(arrangement.rank[&vertex]),
                },
                vertex,
            )
        })
        .collect()
}

/// One polyline is simple: right-angled, through no vertex it does not join,
/// and never over itself (RFC 0002 §8). Direction is not checked here, because
/// a loop return is the one route that climbs.
fn simple(
    points: &[Point],
    vertices: &[(Point, Vertex)],
    joins: &[Vertex],
    what: &str,
) -> Result<(), String> {
    if points.len() < 2 {
        return Err(format!("{what} has no corridor"));
    }
    for segment in points.windows(2) {
        if segment[0].x != segment[1].x && segment[0].y != segment[1].y {
            return Err(format!("{what} bends diagonally"));
        }
        for &(point, vertex) in vertices {
            if joins.contains(&vertex) {
                continue;
            }
            if on_segment(point, segment) {
                return Err(format!("{what} passes through {vertex:?}"));
            }
        }
    }
    if overlaps_itself(points) {
        return Err(format!("{what} overlaps itself"));
    }
    Ok(())
}

/// Routes are simple, never meet a vertex they do not touch, and cross nothing.
fn routes(
    topology: &Topology,
    arrangement: &Arrangement,
    grid: &Grid,
    lines: &[Vec<Point>],
) -> Result<(), String> {
    let vertices = vertex_points(topology, arrangement, grid);

    for (index, points) in lines.iter().enumerate() {
        let wire = topology.connections[index];
        let joins = [Vertex::from(wire.source), wire.destination];
        simple(
            points,
            &vertices,
            &joins,
            &format!("connection {}", index + 1),
        )?;
        for segment in points.windows(2) {
            if segment[1].y < segment[0].y {
                return Err(format!("connection {} moves upward", index + 1));
            }
        }
        // RFC 0002 §8: an incoming side route must not turn down over the
        // continuation the junction's outgoing connection owns.
        if matches!(
            topology.connections[index].destination,
            Destination::Junction(_)
        ) && turns_downward(points)
        {
            return Err(format!(
                "connection {} turns downward before its merge",
                index + 1
            ));
        }
    }

    for left in 0..lines.len() {
        for right in left + 1..lines.len() {
            let (shared, meetings) = meetings(
                ends(topology, left),
                &lines[left],
                ends(topology, right),
                &lines[right],
            );
            if !compatible(&lines[left], &lines[right], shared, &meetings) {
                return Err(format!("connections {} and {} cross", left + 1, right + 1));
            }
        }
    }

    Ok(())
}

/// Each return climbs outside its body, on the side its contour names, and
/// crosses nothing (RFC 0002 §8).
fn returns(
    flow: &Flow,
    topology: &Topology,
    arrangement: &Arrangement,
    grid: &Grid,
    lines: &[Vec<Point>],
) -> Result<(), String> {
    let vertices = vertex_points(topology, arrangement, grid);
    let chosen = arrangement
        .contours
        .iter()
        .map(|contour| Some(*contour))
        .collect::<Vec<_>>();
    let mut drawn: Vec<Vec<Point>> = Vec::new();
    for (index, loop_) in topology.loops.iter().enumerate() {
        let contour = arrangement.contours[index];
        let body = super::loop_block::body_columns(flow, topology, arrangement, loop_.header);
        let nested = super::loop_block::nested_returns(flow, topology, grid, loop_.header, &chosen);
        let position = grid.contour(contour);
        let outside = contour.lane < contour_lanes(topology)
            && super::loop_block::outside(grid, contour.side, position, &body, &nested);
        if !outside {
            return Err(format!(
                "the return of the loop at block {} climbs inside its body",
                loop_.header + 1
            ));
        }
        let line = return_polyline(
            topology,
            arrangement,
            grid,
            loop_.tail,
            loop_.entry,
            contour,
        );
        let back = (
            Source::Junction(loop_.tail),
            Destination::Junction(loop_.entry),
        );
        simple(
            &line,
            &vertices,
            &[Vertex::Junction(loop_.tail), Vertex::Junction(loop_.entry)],
            &format!("the return of the loop at block {}", loop_.header + 1),
        )?;
        for (other, points) in lines.iter().enumerate() {
            let (shared, meetings) = meetings(back, &line, ends(topology, other), points);
            if !compatible(&line, points, shared, &meetings) {
                return Err(format!(
                    "the return of the loop at block {} crosses connection {}",
                    loop_.header + 1,
                    other + 1
                ));
            }
        }
        for earlier in &drawn {
            if !compatible(&line, earlier, false, &[]) {
                return Err(format!(
                    "the return of the loop at block {} crosses another return",
                    loop_.header + 1
                ));
            }
        }
        drawn.push(line);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// How deep a return may climb beside a column is a fact about the
    /// topology, not about any renderer's spacing. A presentation holds
    /// whatever this allows; it does not decide it.
    #[test]
    fn the_lane_bound_counts_loops_rather_than_pixels() {
        let mut topology = Topology::default();
        assert_eq!(contour_lanes(&topology), 0);
        for loops in 1..=8 {
            topology.loops.push(crate::topology::Loop {
                header: loops,
                entry: loops * 2,
                tail: loops * 2 + 1,
                prefer_left: true,
            });
            assert_eq!(contour_lanes(&topology), loops);
        }
    }

    #[test]
    fn shared_entries_keep_the_first_branch_approach_column() {
        for (source, name, entries) in [
            (
                include_str!("../../../kaalang/tests/wire/behavior/independent_entry_blocks.rs"),
                "independent_entry_blocks",
                vec![3, 4],
            ),
            (
                include_str!(
                    "../../../kaalang/tests/wire/behavior/question_after_a_partial_merge.rs"
                ),
                "question_after_a_partial_merge",
                vec![3, 5],
            ),
            (
                include_str!("../../../kaalang/tests/wire/behavior/blocked_terminal_crossing.rs"),
                "blocked_terminal_crossing",
                vec![6],
            ),
            (
                include_str!(
                    "../../../kaalang/tests/wire/behavior/a_branch_captures_a_merged_value.rs"
                ),
                "a_branch_captures_a_merged_value",
                vec![3],
            ),
        ] {
            let model = crate::build(&crate::tests::fixture(source, name)).unwrap();
            let valid = model.arrangement.clone();
            branch_columns(&model.flow, &model.topology, &valid).unwrap();
            for entry in entries {
                let mut moved = valid.clone();
                *moved
                    .column
                    .get_mut(&Vertex::Node(NodeId::Block(entry)))
                    .unwrap() += 1;
                let error = branch_columns(&model.flow, &model.topology, &moved).unwrap_err();
                assert!(
                    error.contains("first branch's approach column"),
                    "{name}, entry {entry}: {error}"
                );
            }
        }
    }
}
