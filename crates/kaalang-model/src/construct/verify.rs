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

/// The position of one column.
pub(super) const fn at_column(column: i32) -> i32 {
    column * SCALE
}

/// The position of one contour lane beside a column. Lane 0 sits immediately
/// outside the column; later lanes step further out.
pub(super) const fn at_contour(contour: super::Contour) -> i32 {
    let offset = contour.lane as i32 + 1;
    match contour.side {
        Side::Left => contour.column * SCALE - offset,
        Side::Right => contour.column * SCALE + offset,
    }
}

/// How many contour lanes a side offers before it reaches the next column.
pub(super) const fn contour_lanes() -> usize {
    SCALE as usize - 1
}

/// How far apart two columns sit on the abstract grid, and so how many return
/// contours fit between them: `SCALE - 1` lanes on the side of each column.
///
/// A presentation has to hold that many. The renderer spaces columns
/// `COLUMN_WIDTH` apart with nodes at most `NODE_WIDTH` wide and steps a
/// contour out by `LANE` at a time, which leaves room for exactly these three.
/// Counting lanes per loop instead would let the search choose a lane no
/// drawing can hold.
const SCALE: i32 = 4;

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
            x: at_column(departure),
            y: source_line,
        },
        Point {
            x: at_column(route.departure),
            y: source_line,
        },
    ];
    for run in &route.runs {
        let line = grid.lane(run.gap, run.lane);
        points.push(Point {
            x: at_column(run.enter),
            y: line,
        });
        points.push(Point {
            x: at_column(run.exit),
            y: line,
        });
    }
    points.push(Point {
        x: at_column(route.arrival),
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
    let aside = at_contour(contour);
    straighten(vec![
        Point {
            x: at_column(arrangement.column[&Vertex::Junction(tail)]),
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
            x: at_column(arrangement.column[&Vertex::Junction(entry)]),
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

/// A selection's branches leave it left to right in authored order, and a
/// brancher's own area stays inside the columns it reserves, so a later sibling
/// starts beyond that area (RFC 0002 §8).
///
/// The columns here come from the placement and the reserved widths from the
/// footprints, which are two separate passes — so this relates one to the other
/// rather than comparing a value with itself.
fn branch_columns(
    flow: &Flow,
    topology: &Topology,
    arrangement: &Arrangement,
) -> Result<(), String> {
    let reachable = super::regions::reachable(topology);
    for block in super::regions::branchers(flow) {
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
        let Some(footprint) = arrangement.footprints.get(&block) else {
            return Err(format!("block {} reserves no columns", block + 1));
        };
        let reserved = own + i32::try_from(footprint.width).unwrap_or(i32::MAX);
        let branches = super::regions::branch_sets(topology, flow, &reachable, block);
        for (entry, first) in super::regions::continuations(topology, block, &branches) {
            // Read the first branch's actual approach, which may have moved
            // through a nested selection since leaving this brancher.
            let mut frontier = vec![entry];
            let mut approaches = Vec::new();
            while let Some(vertex) = frontier.pop() {
                for connection in topology.incoming(vertex) {
                    match connection.source {
                        Source::Junction(junction) => frontier.push(Vertex::Junction(junction)),
                        Source::Exit(exit)
                            if branches[first].contains(&Vertex::Node(exit.node))
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
        for vertex in super::regions::footprint_vertices(topology, block, &branches) {
            let column = arrangement.column[&vertex];
            if column < own || column >= reserved {
                return Err(format!(
                    "block {} draws {vertex:?} in column {column}, outside the columns {own} to {} it reserves",
                    block + 1,
                    reserved - 1
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
                    x: at_column(arrangement.column[&vertex]),
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
    let mut drawn: Vec<Vec<Point>> = Vec::new();
    for (index, loop_) in topology.loops.iter().enumerate() {
        let contour = arrangement.contours[index];
        let nested = arrangement
            .contours
            .iter()
            .map(|contour| Some(*contour))
            .collect::<Vec<_>>();
        let body =
            super::loop_block::body_columns(flow, topology, arrangement, loop_.header, &nested);
        let position = at_contour(contour);
        let outside = contour.lane < contour_lanes()
            && body.iter().all(|&column| match contour.side {
                Side::Left => position < at_column(column),
                Side::Right => position > at_column(column),
            });
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
            let valid = crate::construct(&model.flow, &model.merges, &model.topology).unwrap();
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
