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
use std::collections::BTreeSet;

use crate::model::Flow;
use crate::topology::{Connection, Destination, ExitId, Loop, NodeId, Source, Topology, Vertex};

use super::{Arrangement, RunLine, Side};

/// The abstract grid a arrangement describes. Every rank gets a line of its
/// own and every lane of the gap below it one more; every column gets a
/// position of its own with room beside it for the back edge contours that climb
/// between columns.
///
/// Gap lanes stay above the following occupied rank. Nodes and junctions use
/// the same rank line, independently of their incoming routes.
pub(super) struct Grid {
    step: i32,
    /// How many lanes each rank gap uses.
    lanes: Vec<usize>,
    /// Whether each rank carries a vertex, clear of gap lanes.
    occupied: Vec<bool>,
    /// How far apart two columns sit, so that every contour lane a topology can
    /// need fits between them.
    scale: i32,
}

/// How many lanes one side of a column offers.
///
/// An iteration back edge climbs in the space beside a column, and the cycle
/// boundary that encloses it stands one lane further out. Cycles nested to the
/// same column stack that pair, so a chain of `n` of them reaches `2n`. This is
/// a fact about the topology, not about any presentation's spacing: a renderer
/// holds whatever this many lanes require.
pub(super) fn contour_lanes(topology: &Topology) -> usize {
    2 * topology.loops.len()
}

impl Grid {
    pub(super) fn of(topology: &Topology, arrangement: &Arrangement) -> Self {
        let deepest = arrangement.gap_lanes.iter().copied().max().unwrap_or(0);
        let mut occupied = vec![false; arrangement.ranks + 1];
        for &vertex in &topology.vertices {
            if let Some(&rank) = arrangement.rank.get(&vertex)
                && let Some(carries) = occupied.get_mut(rank)
            {
                *carries = true;
            }
        }
        Self {
            step: 2 + i32::try_from(deepest).unwrap_or(i32::MAX - 2),
            lanes: arrangement.gap_lanes.clone(),
            occupied,
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

    pub(super) fn line(&self, line: RunLine) -> i32 {
        match line {
            RunLine::Rank(rank) => self.rank(rank),
            RunLine::Lane { gap, lane } => self.lane(gap, lane),
        }
    }

    /// The line one lane of one rank gap occupies, packed against the rank
    /// below it.
    ///
    /// A gap outside the arrangement counts as holding this lane alone. Only
    /// `coverage` rejects such a gap, and it runs first, so this decides
    /// nothing for an arrangement the search produces; it keeps the check
    /// reporting rather than panicking on one it never would.
    pub(super) fn lane(&self, gap: usize, lane: usize) -> i32 {
        let capture = i32::from(self.occupied.get(gap + 1).copied().unwrap_or(false));
        let lanes = self.lanes.get(gap).copied().unwrap_or(lane + 1);
        self.rank(gap + 1) - capture - (lanes.saturating_sub(lane + 1)) as i32
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
    let source_line = grid.rank(arrangement.rank[&Vertex::from(wire.source)]);
    let destination_line = grid.rank(arrangement.rank[&wire.destination]);
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
        let line = grid.line(run.line);
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
        y: destination_line,
    });
    straighten(points)
}

/// The iteration back edge of the cycle at `index` as an orthogonal polyline:
/// out of the tail, up the contour, and horizontally into the entry (RFC 0002
/// §8).
pub(super) fn back_edge_polyline(
    topology: &Topology,
    arrangement: &Arrangement,
    grid: &Grid,
    index: usize,
    contour: super::Contour,
) -> Vec<Point> {
    let Loop { tail, entry, .. } = topology.loops[index];
    let tail_line = grid.rank(arrangement.rank[&Vertex::Junction(tail)]);
    let entry_line = grid.rank(arrangement.rank[&Vertex::Junction(entry)]);
    let position = |column| grid.contour(super::Contour { column, ..contour });
    let Some(route) = arrangement.back_routes.get(&index) else {
        return straighten(vec![
            Point {
                x: grid.column(arrangement.column[&Vertex::Junction(tail)]),
                y: tail_line,
            },
            Point {
                x: position(contour.column),
                y: tail_line,
            },
            Point {
                x: position(contour.column),
                y: entry_line,
            },
            Point {
                x: grid.column(arrangement.column[&Vertex::Junction(entry)]),
                y: entry_line,
            },
        ]);
    };
    let mut points = vec![
        Point {
            x: grid.column(arrangement.column[&Vertex::Junction(tail)]),
            y: tail_line,
        },
        Point {
            x: position(route.arrival),
            y: tail_line,
        },
    ];
    for run in route.runs.iter().rev() {
        let y = grid.line(run.line);
        points.extend([
            Point {
                x: position(run.exit),
                y,
            },
            Point {
                x: position(run.enter),
                y,
            },
        ]);
    }
    points.extend([
        Point {
            x: position(route.departure),
            y: entry_line,
        },
        Point {
            x: grid.column(arrangement.column[&Vertex::Junction(entry)]),
            y: entry_line,
        },
    ]);
    straighten(points)
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
    check(
        flow,
        topology,
        arrangement,
        &Shape::of(flow, topology),
        false,
    )
}

/// What the rules read from the flow and its topology alone.
///
/// Branch regions, their reachability closure and the cycle bodies do not
/// depend on the ranks and columns a candidate proposes, and walking them is
/// the most expensive part of a check. Compaction measures thousands of
/// candidates against one topology, so it settles these once.
pub(super) struct Shape {
    bodies: super::loop_block::Bodies,
    branchers: Vec<Brancher>,
}

/// One selection's regions, with the continuation entries it owns and the
/// first-branch exits each of them is approached from.
struct Brancher {
    block: usize,
    regions: super::regions::Regions,
    continuations: Vec<(Vertex, BTreeSet<ExitId>)>,
    /// Per convergence group and later sibling branch, what the group reserves
    /// and what that branch draws outside it. Only the columns these land in
    /// depend on a candidate.
    reservations: Vec<(usize, usize, Vec<Vertex>, Vec<Vertex>)>,
}

impl Shape {
    /// Whether compaction should construct the candidates that lift `edge`'s
    /// destination into the rows of the body it leaves.
    pub(super) fn may_rise_beside(&self, arrangement: &Arrangement, edge: &Connection) -> bool {
        self.bodies.may_rise_beside(arrangement, edge)
    }

    pub(super) fn of(flow: &Flow, topology: &Topology) -> Self {
        let reachable = super::regions::reachable(topology);
        Self {
            bodies: super::loop_block::Bodies::of(flow, topology),
            branchers: super::regions::branchers(flow, topology)
                .into_iter()
                .map(|block| {
                    let regions = super::regions::regions(flow, topology, &reachable, block);
                    let continuations =
                        super::regions::continuations(topology, block, &regions.branches)
                            .into_iter()
                            .map(|(entry, first)| {
                                let approaches = super::regions::first_branch_approaches(
                                    topology,
                                    &regions.branches,
                                    block,
                                    first,
                                    entry,
                                );
                                (entry, approaches)
                            })
                            .collect();
                    let mut reservations = Vec::new();
                    for (index, group) in regions.groups.iter().enumerate() {
                        for branch in regions.later_siblings(group) {
                            reservations.push((
                                index,
                                branch,
                                regions.outside(group, branch).into_iter().collect(),
                                regions.reserved(group, branch).into_iter().collect(),
                            ));
                        }
                    }
                    Brancher {
                        block,
                        regions,
                        continuations,
                        reservations,
                    }
                })
                .collect(),
        }
    }
}

/// The rules `placement` does not cover, over an arrangement that also draws
/// every cycle boundary.
///
/// The two are a pair: compaction runs `placement` on a candidate before
/// compressing it and this on the result, so between them every rule holds.
///
/// A construction places a cycle's completion below the whole body, which keeps
/// it out of the boundary without ever measuring the rectangle. Compaction
/// measures it: `loop_block::boundaries` reads the rectangle directly, so a
/// completion beside the body may share its rows and rise. The two go together
/// — the relaxed row rule is only sound because the boundary rule holds.
///
/// # Errors
///
/// Returns the rule and the items that break it.
pub(super) fn compacted(
    flow: &Flow,
    topology: &Topology,
    arrangement: &Arrangement,
    shape: &Shape,
) -> Result<(), String> {
    check(flow, topology, arrangement, shape, true)
}

/// The rules that read only ranks and columns, which a candidate satisfies or
/// not whether or not its coordinates have been compressed.
///
/// Compression renumbers columns and lanes onto a dense range; it preserves
/// every order and equality these two rules read. Compaction asks them first,
/// so the candidates they refuse never pay for the compression or the geometry.
pub(super) fn placement(
    topology: &Topology,
    arrangement: &Arrangement,
    shape: &Shape,
) -> Result<(), String> {
    super::end::verify(topology, arrangement)?;
    order(topology, arrangement, Some(&shape.bodies))?;
    serial_columns(topology, arrangement)
}

fn check(
    flow: &Flow,
    topology: &Topology,
    arrangement: &Arrangement,
    shape: &Shape,
    compacting: bool,
) -> Result<(), String> {
    coverage(topology, arrangement)?;
    super::choice::verify(topology, arrangement)?;
    if !compacting {
        super::end::verify(topology, arrangement)?;
        order(topology, arrangement, None)?;
        serial_columns(topology, arrangement)?;
    }
    let grid = Grid::of(topology, arrangement);
    let lines = (0..topology.connections.len())
        .map(|index| polyline(topology, arrangement, &grid, index))
        .collect::<Vec<_>>();
    // The relaxed row rule and the boundary rule go together: the rows a
    // completion may share are exactly the ones the rectangle leaves it, so
    // both hold or neither does. Sorting a candidate by ownership is cheaper
    // than measuring every crossing in it, so this answers first and the
    // geometry rules see fewer.
    if compacting {
        super::loop_block::boundaries(topology, arrangement, &grid, &lines, &shape.bodies)?;
    }
    routes(topology, arrangement, &grid, &lines)?;
    back_edges(flow, topology, arrangement, &grid, &lines)?;
    branch_columns(flow, arrangement, shape)
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
        return Err("the arrangement covers a different number of iteration back edges".to_owned());
    }
    if arrangement.gap_lanes.len() != arrangement.ranks {
        return Err("the arrangement counts lanes for a different number of rank gaps".to_owned());
    }
    for (index, route) in arrangement.all_routes().enumerate() {
        for run in &route.runs {
            match run.line {
                RunLine::Rank(rank) if rank >= arrangement.ranks => {
                    return Err(format!(
                        "connection {} runs on an absent rank {rank}",
                        index + 1
                    ));
                }
                RunLine::Lane { gap, lane } => {
                    let lanes = arrangement.gap_lanes.get(gap).copied().unwrap_or(0);
                    if lane >= lanes {
                        return Err(format!(
                            "connection {} takes lane {lane} of a rank gap with {lanes}",
                            index + 1,
                        ));
                    }
                }
                RunLine::Rank(_) => {}
            }
        }
    }
    for (&index, route) in &arrangement.back_routes {
        let Some(contour) = arrangement.contours.get(index) else {
            return Err("an iteration back edge names no cycle".to_owned());
        };
        if route.departure != contour.column {
            return Err("an iteration back edge changes its recorded entry column".to_owned());
        }
    }
    // A corridor has to end at the vertices its connection joins, and leave by
    // a column that exit owns: either its own branch column, a case column its
    // select distributor reaches sideways, or the column of the merge a later
    // question branch joins at once (RFC 0002 §8).
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
                if let Some(case) = super::choice::case_destination(wire.source, wire.destination) {
                    route.departure == arrangement.column[&Vertex::Node(case)]
                } else {
                    // A later question branch may join its merge column at
                    // once, which means turning towards it: the merge is left
                    // of the branch's own column, never right of it.
                    let merge = arrangement.column[&wire.destination];
                    let shortcut = matches!(wire.destination, Destination::Junction(_))
                        && exit.branch.is_some_and(|branch| branch > 0)
                        && merge < own
                        && route.departure == merge;
                    route.departure == own || shortcut
                }
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

/// Forward connections descend, except for a side exit that ends at a wire merge
/// or its sole iteration tail on its row. A cycle's own tail and result may share
/// a row when their routes are disjoint; a cycle's completion is held out of the
/// body by `boundaries` rather than by its row; other placement-only relations
/// descend.
fn order(
    topology: &Topology,
    arrangement: &Arrangement,
    drawn: Option<&super::loop_block::Bodies>,
) -> Result<(), String> {
    for (relation, edges, same_row) in [
        ("a connection", &topology.connections, true),
        ("placement precedence", &topology.order, false),
    ] {
        for edge in edges {
            let from = arrangement.rank[&Vertex::from(edge.source)];
            let to = arrangement.rank[&edge.destination];
            let aligned = if same_row {
                topology.same_row_junction(edge)
            } else {
                topology.loops.iter().any(|loop_| {
                    edge.source == Source::Junction(loop_.tail)
                        && topology.loop_boundaries.iter().any(|boundary| {
                            boundary.header == loop_.header
                                && boundary.result.map(Vertex::from) == Some(edge.destination)
                        })
                })
            };
            if (from > to || (from == to && !aligned))
                && (same_row || !drawn.is_some_and(|bodies| bodies.completes_a_boundary(edge)))
            {
                return Err(format!("{relation} does not descend: {from} to {to}"));
            }
        }
    }
    Ok(())
}

/// A sole arrival continues the current column. A case may be reached by a
/// distributor detour; a tail may finish at either end of its arrival rail.
fn serial_columns(topology: &Topology, arrangement: &Arrangement) -> Result<(), String> {
    for &vertex in &topology.vertices {
        let Some(wire) = super::serial_arrival(topology, vertex) else {
            continue;
        };
        let column = match wire.source {
            Source::Exit(exit) => {
                arrangement.column[&Vertex::Node(exit.node)] + arrangement.exit_offset[&exit]
            }
            Source::Junction(junction) => arrangement.column[&Vertex::Junction(junction)],
        };
        if arrangement.column[&vertex] != column {
            return Err(format!("{vertex:?} leaves its serial column {column}"));
        }
    }
    Ok(())
}

/// A selection's branches leave it left to right in authored order, its shared
/// continuations sit in the column their group's first branch reached, and
/// each convergence group keeps the columns it reserves (RFC 0002 §8).
fn branch_columns(flow: &Flow, arrangement: &Arrangement, shape: &Shape) -> Result<(), String> {
    for brancher in &shape.branchers {
        let block = brancher.block;
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
        for (entry, approaches) in &brancher.continuations {
            let entry = *entry;
            let column = approaches
                .iter()
                .map(|exit| {
                    arrangement.column[&Vertex::Node(exit.node)] + arrangement.exit_offset[exit]
                })
                .min()
                .ok_or_else(|| {
                    format!("continuation entry {entry:?} has no first-branch approach")
                })?;
            if arrangement.column[&entry] != column {
                return Err(format!(
                    "block {} draws continuation entry {entry:?} away from its first branch's approach column {column}",
                    block + 1,
                ));
            }
        }
        reserved_columns(arrangement, brancher)?;
    }
    Ok(())
}

/// A convergence group reserves the columns of everything its branches draw,
/// and a sibling written after every member of it starts to their right (RFC
/// 0002 §8).
///
/// This is the same comparison the deciding search's numbering records, vertex
/// by vertex, over the same set: what the group draws and the sibling does not,
/// which `Regions::reserved` derives from the topology, so this holds an
/// arrangement to an order it did not choose. Reading a width the same search
/// recorded would compare a value with itself.
///
/// A vertex two branches reach is a convergence and belongs to the area of the
/// group that meets there; a vertex the sibling reaches too is common ground on
/// both sides of the comparison, so `Regions::reserved` takes it out as
/// `Regions::outside` takes it out of the branch. Leaving it in would make the
/// rule unsatisfiable for the first branch, whose column RFC 0002 §8 fixes as
/// the brancher's own.
///
/// Only a later sibling is held. An earlier sibling and one the group encloses
/// are held to nothing — RFC 0003 §2.2 records the mirror-image restriction
/// that was tried and the flow it refused — and neither are branches that never
/// meet: RFC 0002 §8 reserves columns for a convergence group, not for every
/// branch, so their subtrees may interleave.
fn reserved_columns(arrangement: &Arrangement, brancher: &Brancher) -> Result<(), String> {
    let column = |vertex: &Vertex| arrangement.column[vertex];
    for (group, branch, outside, reserved) in &brancher.reservations {
        for inside in reserved {
            let at = column(inside);
            let Some(wrong) = outside.iter().find(|vertex| column(vertex) <= at) else {
                continue;
            };
            return Err(format!(
                "block {} draws {wrong:?} of branch {} in column {}, not right of {inside:?} in column {at}, which its convergence group {:?} reserves",
                brancher.block + 1,
                branch + 1,
                column(wrong),
                brancher.regions.groups[*group].members
            ));
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
/// an iteration back edge is the one route that climbs.
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
        if matches!(wire.destination, Destination::Junction(_)) && turns_downward(points) {
            return Err(format!(
                "connection {} turns downward before its merge",
                index + 1
            ));
        }
    }

    if let Some((left, right)) = crossing(topology, lines) {
        return Err(format!("connections {} and {} cross", left + 1, right + 1));
    }

    Ok(())
}

/// The first pair of connections whose polylines cross, including a descent
/// through another question's side departure. Bundles may share segments, but
/// the complete polylines must still meet the ordinary crossing rules.
pub(super) fn crossing(topology: &Topology, lines: &[Vec<Point>]) -> Option<(usize, usize)> {
    // Most pairs in a long flow occupy disjoint vertical intervals. Reject
    // those pairs before computing bundle meetings and comparing segments.
    let spans = lines
        .iter()
        .map(|line| {
            line.iter()
                .fold((i32::MAX, i32::MIN), |(low, high), point| {
                    (low.min(point.y), high.max(point.y))
                })
        })
        .collect::<Vec<_>>();
    for (left, line) in lines.iter().enumerate() {
        for (right, other) in lines.iter().enumerate().skip(left + 1) {
            if spans[left].1 < spans[right].0 || spans[right].1 < spans[left].0 {
                continue;
            }
            let (shared, allowed) =
                meetings(ends(topology, left), line, ends(topology, right), other);
            if !compatible(line, other, shared, &allowed) {
                return Some((left, right));
            }
        }
    }
    None
}

/// Each iteration back edge climbs outside its body, on the side its contour names, and
/// crosses nothing (RFC 0002 §8).
fn back_edges(
    flow: &Flow,
    topology: &Topology,
    arrangement: &Arrangement,
    grid: &Grid,
    lines: &[Vec<Point>],
) -> Result<(), String> {
    let vertices = vertex_points(topology, arrangement, grid);
    let back_edges = (0..topology.loops.len())
        .map(|i| back_edge_polyline(topology, arrangement, grid, i, arrangement.contours[i]))
        .collect::<Vec<_>>();
    let mut drawn: Vec<&Vec<Point>> = Vec::new();
    for (index, loop_) in topology.loops.iter().enumerate() {
        let contour = arrangement.contours[index];
        let body = super::loop_block::body_columns(flow, topology, arrangement, loop_.header);
        let end = flow.blocks[loop_.header]
            .loop_end
            .expect("a loop owns a body");
        let nested = topology
            .loops
            .iter()
            .enumerate()
            .filter(|(_, inner)| (loop_.header + 1..end).contains(&inner.header))
            .flat_map(|(i, _)| back_edges[i].iter().map(|p| p.x))
            .collect::<Vec<_>>();
        let line = &back_edges[index];
        let outside = contour.lane < contour_lanes(topology)
            && line.len() >= 4
            && line[1..line.len() - 1]
                .iter()
                .all(|p| super::loop_block::outside(grid, contour.side, p.x, &body, &nested));
        if !outside {
            return Err(format!(
                "the iteration back edge of the cycle at block {} climbs inside its body",
                loop_.header + 1
            ));
        }
        if line.windows(2).any(|pair| pair[1].y > pair[0].y) {
            return Err(format!(
                "the iteration back edge of the cycle at block {} moves downward",
                loop_.header + 1
            ));
        }
        let back = (
            Source::Junction(loop_.tail),
            Destination::Junction(loop_.entry),
        );
        simple(
            line,
            &vertices,
            &[Vertex::Junction(loop_.tail), Vertex::Junction(loop_.entry)],
            &format!(
                "the iteration back edge of the cycle at block {}",
                loop_.header + 1
            ),
        )?;
        for (other, points) in lines.iter().enumerate() {
            let (shared, meetings) = meetings(back, line, ends(topology, other), points);
            if !compatible(line, points, shared, &meetings) {
                return Err(format!(
                    "the iteration back edge of the cycle at block {} crosses connection {}",
                    loop_.header + 1,
                    other + 1
                ));
            }
        }
        for earlier in &drawn {
            if !compatible(line, earlier, false, &[]) {
                return Err(format!(
                    "the iteration back edge of the cycle at block {} crosses another back edge",
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

    /// Check reserved columns directly: moving the sibling also breaks its
    /// serial column, which the complete verifier may reject first.
    #[test]
    fn a_sibling_inside_a_reserved_footprint_is_caught() {
        let source = super::super::tests::looping(&["repeat", "repeat", "break", "break"]);
        let model = crate::build(&syn::parse_str(&source).unwrap()).unwrap();
        let reachable = super::super::regions::reachable(&model.topology);
        let block = super::super::regions::branchers(&model.flow, &model.topology)[0];
        let regions =
            super::super::regions::regions(&model.flow, &model.topology, &reachable, block);
        let group = regions
            .groups
            .iter()
            .find(|group| group.members.len() == 2)
            .expect("two routes repeat, so they converge");
        let last = regions.branches.len() - 1;
        let head = Vertex::Node(NodeId::Case {
            choice: block,
            branch: last,
        });
        let outside = regions
            .outside(group, last)
            .into_iter()
            .filter(|vertex| *vertex != head)
            .collect::<Vec<_>>();
        assert!(
            !outside.is_empty(),
            "the exit branch has work outside the repeating group's footprint"
        );
        let inside = group
            .area
            .iter()
            .map(|vertex| model.arrangement.column[vertex])
            .max()
            .expect("the group draws something");

        branch_columns(
            &model.flow,
            &model.arrangement,
            &Shape::of(&model.flow, &model.topology),
        )
        .unwrap();
        let mut broken = model.arrangement.clone();
        for vertex in outside {
            broken.column.insert(vertex, inside);
        }
        let reason = branch_columns(
            &model.flow,
            &broken,
            &Shape::of(&model.flow, &model.topology),
        )
        .expect_err("a sibling inside the reserved columns does not conform");
        assert!(
            reason.contains("its convergence group"),
            "the reserved columns should be the rule that objects: {reason}"
        );
    }

    #[test]
    fn a_crossing_uses_the_structural_junctions_rank() {
        let junction = Vertex::Junction(0);
        let mut topology = Topology::default();
        topology.vertices = vec![junction];
        topology.connections = vec![crate::topology::Connection {
            source: Source::Junction(1),
            destination: junction,
        }];
        let arrangement = Arrangement {
            ranks: 3,
            rank: [(junction, 2)].into(),
            column: [(junction, 0)].into(),
            gap_lanes: vec![0, 2, 0],
            routes: vec![super::super::Route {
                departure: -1,
                arrival: 0,
                runs: vec![super::super::Run {
                    line: RunLine::Lane { gap: 1, lane: 0 },
                    enter: -1,
                    exit: 0,
                }],
            }],
            ..Arrangement::default()
        };
        let grid = Grid::of(&topology, &arrangement);
        let y = grid.rank(arrangement.rank[&junction]);
        assert!(
            grid.lane(1, 0) < y,
            "a routing lane stays above the junction"
        );
        let crossing = [Point { x: -2, y }, Point { x: 2, y }];
        assert!(
            simple(
                &crossing,
                &vertex_points(&topology, &arrangement, &grid),
                &[],
                "probe"
            )
            .unwrap_err()
            .contains("passes through")
        );
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
                vec![7],
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
            branch_columns(
                &model.flow,
                &valid,
                &Shape::of(&model.flow, &model.topology),
            )
            .unwrap();
            for entry in entries {
                let mut moved = valid.clone();
                *moved
                    .column
                    .get_mut(&Vertex::Node(NodeId::Block(entry)))
                    .unwrap() += 1;
                let error = branch_columns(
                    &model.flow,
                    &moved,
                    &Shape::of(&model.flow, &model.topology),
                )
                .unwrap_err();
                assert!(
                    error.contains("first branch's approach column"),
                    "{name}, entry {entry}: {error}"
                );
            }
        }
    }
}
