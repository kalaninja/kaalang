//! Decides the contour of every iteration back edge.
//!
//! RFC 0002 §8 sends a back edge upward outside its body and horizontally into its
//! entry junction, and RFC 0002 §7 keeps a break's first wire merge or end below
//! the body it leaves. Only a sole side exit may reach an enclosing tail
//! without carrying the nested body's precedence.
//! A contour therefore needs a column outside the body that no route occupies
//! over the back edge's whole rank span, and two horizontal runs that meet nothing.
//!
//! The preferred search tries the lanes beside the body's edge, nearest first.
//! Exhausting them leaves the complete constructor available; it does not
//! establish that the topology is impossible.
//!
//! RFC 0002 §8 only *prefers* a side, so a blocked preferred side never decides
//! realizability: the caller hands this module the side to try and flips it
//! when no contour holds.

use std::collections::BTreeSet;

use crate::model::{Flow, WireMerge};
use crate::topology::{Connection, Destination, NodeId, Source, Topology, Vertex};

use crate::geometry::{Point, compatible, enters, inside};

use super::verify::{Grid, back_edge_polyline, ends, meetings, polyline};
use super::{Arrangement, Contour, Side};

/// The columns one loop's whole body occupies, independently of its ranks.
/// Moving a body vertex below the tail does not remove it from the body, and
/// shortening a back edge does not shrink the extent it has to clear.
///
/// A back edge nested inside this body is not one of them. It climbs beside the
/// body in a lane of its own, and an enclosing back edge takes the next lane out
/// rather than a whole column. `nested_back_edges` is what holds the two apart:
/// a crossing check cannot, because the rows they span need not overlap.
pub(super) fn body_columns(
    flow: &Flow,
    topology: &Topology,
    arrangement: &Arrangement,
    header: usize,
) -> BTreeSet<i32> {
    body_vertices(flow, topology, header)
        .into_iter()
        .map(|vertex| arrangement.column[&vertex])
        .collect()
}

/// What every cycle boundary encloses, in `Topology::loop_boundaries` order.
///
/// A boundary's contents depend on the flow and its topology, never on the
/// candidate ranks and columns, and settling one walks a fixpoint over the
/// junctions it draws. One check asks for them once and hands them to every
/// rule that reads them; compaction checks thousands of candidates against the
/// same contents.
pub(super) struct Bodies(Vec<Body>);

/// One boundary's contents, with everything the rules sort by ownership rather
/// than by position: which vertices stand outside it, which routes stay inside,
/// which reach its interface, and which back edges it draws.
struct Body {
    owned: BTreeSet<Vertex>,
    foreign: Vec<Vertex>,
    internal: Vec<usize>,
    foreign_routes: Vec<usize>,
    /// The cycle's own iteration back edge, which its boundary encloses.
    own_back_edge: Option<usize>,
    foreign_loops: Vec<usize>,
    /// The other boundaries this one holds, and the ones that hold it.
    nests: Vec<usize>,
    beside: Vec<usize>,
}

impl Bodies {
    pub(super) fn of(flow: &Flow, topology: &Topology) -> Self {
        Self(
            topology
                .loop_boundaries
                .iter()
                .map(|boundary| {
                    let owned = owned(flow, topology, boundary.header);
                    let inside = |vertex: Vertex| owned.contains(&vertex);
                    let interface = |edge: &Connection| {
                        edge.destination == boundary.entry || boundary.result == Some(edge.source)
                    };
                    let (mut internal, mut foreign_routes) = (Vec::new(), Vec::new());
                    for (index, edge) in topology.connections.iter().enumerate() {
                        let both = inside(Vertex::from(edge.source)) && inside(edge.destination);
                        if both {
                            internal.push(index);
                        } else if !interface(edge) {
                            foreign_routes.push(index);
                        }
                    }
                    let mut foreign_loops = Vec::new();
                    for (index, loop_) in topology.loops.iter().enumerate() {
                        if !(boundary.header..boundary.end).contains(&loop_.header) {
                            foreign_loops.push(index);
                        }
                    }
                    let own_back_edge = topology
                        .loops
                        .iter()
                        .position(|loop_| loop_.header == boundary.header);
                    let (mut nests, mut beside) = (Vec::new(), Vec::new());
                    for (index, other) in topology.loop_boundaries.iter().enumerate() {
                        if other.header == boundary.header {
                            continue;
                        }
                        if (boundary.header + 1..boundary.end).contains(&other.header) {
                            nests.push(index);
                        } else if !(other.header + 1..other.end).contains(&boundary.header) {
                            beside.push(index);
                        }
                    }
                    Body {
                        foreign: topology
                            .vertices
                            .iter()
                            .copied()
                            .filter(|vertex| !owned.contains(vertex))
                            .collect(),
                        owned,
                        internal,
                        foreign_routes,
                        own_back_edge,
                        foreign_loops,
                        nests,
                        beside,
                    }
                })
                .collect(),
        )
    }

    /// Whether lifting `edge`'s destination past the body could ever hold.
    ///
    /// A completion that stands in one of the body's own columns has no row
    /// beside it: every row the body spans is inside the rectangle. Compaction
    /// asks this before constructing the candidates, which the boundary rule
    /// would reject one by one.
    pub(super) fn may_rise_beside(&self, arrangement: &Arrangement, edge: &Connection) -> bool {
        let landing = arrangement.column[&edge.destination];
        self.0.iter().any(|body| {
            if !body.owned.contains(&Vertex::from(edge.source))
                || body.owned.contains(&edge.destination)
            {
                return false;
            }
            let mut columns = body.owned.iter().map(|vertex| arrangement.column[vertex]);
            let first = columns.next().unwrap_or(landing);
            let (left, right) = columns.fold((first, first), |(left, right), column| {
                (left.min(column), right.max(column))
            });
            landing < left || landing > right
        })
    }

    /// Whether `edge` orders a cycle's completion after the body it leaves.
    ///
    /// `topology/loop_block.rs::order_boundaries` puts the completion below the
    /// whole body so a construction has a conforming arrangement to start from.
    /// That is a starting point, not the rule: RFC 0002 §8 only asks the
    /// boundary to stay clear of what the cycle does not own, and `boundaries`
    /// checks that rectangle directly. A completion beside the body may
    /// therefore share its rows, which is what lets compaction lift it.
    pub(super) fn completes_a_boundary(&self, edge: &Connection) -> bool {
        self.0.iter().any(|body| {
            body.owned.contains(&Vertex::from(edge.source))
                && !body.owned.contains(&edge.destination)
        })
    }
}

/// Whether one rectangle holds another whole.
const fn contains(
    (left, top, right, bottom): (i32, i32, i32, i32),
    (other_left, other_top, other_right, other_bottom): (i32, i32, i32, i32),
) -> bool {
    left <= other_left && top <= other_top && right >= other_right && bottom >= other_bottom
}

/// Whether two rectangles share any area.
const fn overlaps(
    (left, top, right, bottom): (i32, i32, i32, i32),
    (other_left, other_top, other_right, other_bottom): (i32, i32, i32, i32),
) -> bool {
    left < other_right && other_left < right && top < other_bottom && other_top < bottom
}

/// A cycle boundary encloses its own body and nothing else (RFC 0002 §8).
///
/// This is the rule the blanket precedence used to stand in for. A vertex or a
/// route the cycle does not own may share the body's rows, as long as it stays
/// out of the rectangle. Only the cycle's interface reaches it: the route into
/// its entry and the one leaving its result.
pub(super) fn boundaries(
    topology: &Topology,
    arrangement: &Arrangement,
    grid: &Grid,
    lines: &[Vec<Point>],
    bodies: &Bodies,
) -> Result<(), String> {
    if topology.loop_boundaries.is_empty() {
        return Ok(());
    }
    let backs = (0..topology.loops.len())
        .map(|index| {
            back_edge_polyline(
                topology,
                arrangement,
                grid,
                index,
                arrangement.contours[index],
            )
        })
        .collect::<Vec<_>>();
    // Every rule below reads the same corridors and cells; measuring each of
    // them once and folding the extents is what keeps a boundary check cheap
    // enough for compaction to run one per candidate.
    let extents = |points: &[Point]| {
        let first = points.first().copied().unwrap_or(Point { x: 0, y: 0 });
        points.iter().fold(
            (first.x, first.y, first.x, first.y),
            |(left, top, right, bottom), point| {
                (
                    left.min(point.x),
                    top.min(point.y),
                    right.max(point.x),
                    bottom.max(point.y),
                )
            },
        )
    };
    let route_extents = lines.iter().map(|line| extents(line)).collect::<Vec<_>>();
    let back_extents = backs.iter().map(|line| extents(line)).collect::<Vec<_>>();
    let drawn = rectangles(
        topology,
        arrangement,
        grid,
        bodies,
        &route_extents,
        &back_extents,
    );
    let named = |index: usize| topology.loop_boundaries[index].header + 1;
    for (index, body) in bodies.0.iter().enumerate() {
        let bounds = drawn[index];
        for &other in &body.nests {
            if !contains(bounds, drawn[other]) {
                return Err(format!(
                    "the boundary of the cycle at block {} does not hold the one at block {}",
                    named(index),
                    named(other)
                ));
            }
        }
        for &other in &body.beside {
            if overlaps(bounds, drawn[other]) {
                return Err(format!(
                    "the boundaries of the cycles at blocks {} and {} overlap",
                    named(index),
                    named(other)
                ));
            }
        }
        for &vertex in &body.foreign {
            let point = Point {
                x: grid.column(arrangement.column[&vertex]),
                y: grid.rank(arrangement.rank[&vertex]),
            };
            if inside(point, bounds) {
                return Err(format!(
                    "the cycle at block {} encloses {vertex:?}",
                    named(index)
                ));
            }
        }
        for &route in &body.foreign_routes {
            if lines[route]
                .windows(2)
                .any(|segment| enters(segment[0], segment[1], bounds))
            {
                let edge = topology.connections[route];
                return Err(format!(
                    "the cycle at block {} is crossed by {:?} -> {:?}, a route it does not own",
                    named(index),
                    edge.source,
                    edge.destination
                ));
            }
        }
        for &loop_ in &body.foreign_loops {
            if backs[loop_]
                .windows(2)
                .any(|segment| enters(segment[0], segment[1], bounds))
            {
                return Err(format!(
                    "the iteration back edge of the cycle at block {} enters the boundary of the one at block {}",
                    topology.loops[loop_].header + 1,
                    named(index)
                ));
            }
        }
    }
    Ok(())
}

/// The rectangle each cycle boundary draws, in `Topology::loop_boundaries`
/// order, as `(left, top, right, bottom)` on the abstract grid.
///
/// RFC 0002 §8 makes the boundary enclose every vertex, junction and back edge
/// the cycle owns, and nothing else. It therefore stands one lane outside all
/// of them: the body's own cells and internal routes, its back edge, and the
/// rectangle of every cycle nested in it. A nested header always follows its
/// enclosing one, so descending header order settles the inner rectangles
/// first.
///
/// The lane budget holds this: `contour_lanes` offers two lanes per cycle, one
/// for a back edge and the next for the boundary around it, so a chain of
/// cycles stacked against one column still stops short of the next column.
fn rectangles(
    topology: &Topology,
    arrangement: &Arrangement,
    grid: &Grid,
    bodies: &Bodies,
    route_extents: &[(i32, i32, i32, i32)],
    back_extents: &[(i32, i32, i32, i32)],
) -> Vec<(i32, i32, i32, i32)> {
    let mut settled = vec![None; topology.loop_boundaries.len()];
    let mut inward = (0..topology.loop_boundaries.len()).collect::<Vec<_>>();
    inward.sort_by_key(|&index| std::cmp::Reverse(topology.loop_boundaries[index].header));
    for index in inward {
        let body = &bodies.0[index];
        let cells = body.owned.iter().map(|vertex| {
            let at = Point {
                x: grid.column(arrangement.column[vertex]),
                y: grid.rank(arrangement.rank[vertex]),
            };
            (at.x, at.y, at.x, at.y)
        });
        let held = cells
            .chain(body.internal.iter().map(|&route| route_extents[route]))
            .chain(body.own_back_edge.iter().map(|&loop_| back_extents[loop_]))
            .chain(body.nests.iter().filter_map(|&nested| settled[nested]));
        let (left, top, right, bottom) = held
            .reduce(|a, b| (a.0.min(b.0), a.1.min(b.1), a.2.max(b.2), a.3.max(b.3)))
            .unwrap_or((0, 0, 0, 0));
        settled[index] = Some((left - 1, top - 1, right + 1, bottom + 1));
    }
    settled
        .into_iter()
        .map(|rectangle| rectangle.expect("every boundary settles once"))
        .collect()
}

/// The vertices one cycle's boundary encloses: its body and the result it
/// hands over.
pub(super) fn owned(flow: &Flow, topology: &Topology, header: usize) -> BTreeSet<Vertex> {
    let mut body = body_vertices(flow, topology, header);
    body.extend(
        topology
            .loop_boundaries
            .iter()
            .filter(|boundary| boundary.header == header)
            .filter_map(|boundary| boundary.result.map(Vertex::from)),
    );
    body
}

/// The vertices one loop's body draws: its own blocks and their cases,
/// together with its entry and tail and those of the loops nested in it.
pub(super) fn body_vertices(flow: &Flow, topology: &Topology, header: usize) -> BTreeSet<Vertex> {
    let end = flow.blocks[header].loop_end.expect("a loop owns a body");
    let body = header + 1..end;
    let result = topology
        .loop_boundaries
        .iter()
        .find(|boundary| boundary.header == header)
        .and_then(|boundary| boundary.result);
    let mut vertices = BTreeSet::new();
    for node in &topology.nodes {
        let block = match node.id {
            NodeId::Block(block) => block,
            NodeId::Case { choice, .. } => choice,
            NodeId::Start => continue,
        };
        if body.contains(&block) {
            vertices.insert(Vertex::Node(node.id));
        }
    }
    for loop_ in &topology.loops {
        if loop_.header == header || body.contains(&loop_.header) {
            vertices.insert(Vertex::Junction(loop_.entry));
            vertices.insert(Vertex::Junction(loop_.tail));
        }
    }
    for boundary in &topology.loop_boundaries {
        if boundary.header == header {
            vertices.insert(boundary.entry);
            vertices.extend(boundary.result.map(Vertex::from).filter(|result| {
                matches!(result, Vertex::Junction(junction)
                        if !topology.junctions[*junction].merges.is_empty())
            }));
        } else if body.contains(&boundary.header) {
            vertices.insert(boundary.entry);
            vertices.extend(boundary.result.map(Vertex::from));
        }
    }
    // A wire merge or a break inside the body draws a junction and no node, so
    // the blocks alone miss it. Everything reaching such a junction comes from
    // the body, and a chain of them needs more than one pass.
    let mut settled = false;
    while !settled {
        settled = true;
        for junction in 0..topology.junctions.len() {
            let vertex = Vertex::Junction(junction);
            if vertices.contains(&vertex) {
                continue;
            }
            // Cycle interfaces and tails belong to their own lexical cycle.
            // Those of this body were inserted above; reaching an enclosing
            // boundary or a following cycle does not make that continuation
            // part of the body.
            if topology
                .loops
                .iter()
                .any(|loop_| junction == loop_.entry || junction == loop_.tail)
                || topology.junctions[junction].is_loop_result
            {
                continue;
            }
            let mut arrivals = topology.incoming(vertex).peekable();
            if arrivals.peek().is_some()
                && arrivals.all(|edge| {
                    Some(edge.source) != result && vertices.contains(&Vertex::from(edge.source))
                })
            {
                vertices.insert(vertex);
                settled = false;
            }
        }
    }
    vertices
}

/// Where the back edges nested inside one body already climb.
///
/// An enclosing back edge has to stay outside those too, and a crossing check
/// cannot see one whose rows do not overlap its own: the two back edges may share
/// no row at all and still leave the enclosing one inside the body it is
/// supposed to clear. The position carries the nested back edge's lane, so the
/// enclosing one only needs the next lane out, not a column of its own.
pub(super) fn nested_back_edges(
    flow: &Flow,
    topology: &Topology,
    grid: &Grid,
    header: usize,
    chosen: &[Option<Contour>],
) -> Vec<i32> {
    let end = flow.blocks[header].loop_end.expect("a loop owns a body");
    topology
        .loops
        .iter()
        .enumerate()
        .filter(|(_, loop_)| (header + 1..end).contains(&loop_.header))
        .filter_map(|(index, _)| chosen.get(index).copied().flatten())
        .map(|contour| grid.contour(contour))
        .collect()
}

/// Whether one contour climbs outside everything its body occupies: every
/// column of the body, and every back edge nested inside it (RFC 0002 §8).
pub(super) fn outside(
    grid: &Grid,
    side: Side,
    position: i32,
    body: &BTreeSet<i32>,
    nested: &[i32],
) -> bool {
    let clears = |other: i32| match side {
        Side::Left => position < other,
        Side::Right => position > other,
    };
    body.iter().all(|&column| clears(grid.column(column))) && nested.iter().copied().all(clears)
}

/// The nearest contour one back edge can climb, or why the nearest candidate
/// failed and the connection to blame, when one is, so the caller can give it
/// more room and try again.
///
/// The candidates are the lanes beside the body's own edge column, nearest
/// first, and nothing beyond them. A lane is the whole of a contour's
/// position, which is what a presentation can realize: it measures the
/// body's boxes and steps that many lanes clear of them, having no column
/// of its own to put a rail in. A topology never needs more lanes than it
/// has cycles, because only a back edge climbs there (RFC 0003 §2.4).
#[allow(clippy::too_many_arguments)]
fn climb(
    flow: &Flow,
    merges: &[WireMerge],
    topology: &Topology,
    arrangement: &Arrangement,
    grid: &Grid,
    lines: &[Vec<Point>],
    drawn: &[Vec<Point>],
    body: &BTreeSet<i32>,
    nested: &[i32],
    index: usize,
    side: Side,
) -> Result<(Contour, Vec<Point>), (String, Option<usize>)> {
    let loop_ = topology.loops[index];
    let edge = match side {
        Side::Left => body.iter().min(),
        Side::Right => body.iter().max(),
    }
    .copied()
    .expect("a repeating cycle owns an entry and a tail");
    let back = (
        Source::Junction(loop_.tail),
        Destination::Junction(loop_.entry),
    );
    let mut blocked = String::new();
    let mut culprit = None;
    for lane in 0..super::verify::contour_lanes(topology) {
        let contour = Contour {
            side,
            column: edge,
            lane,
        };
        if !outside(grid, side, grid.contour(contour), body, nested) {
            "it would climb inside the body it leaves".clone_into(&mut blocked);
            culprit = None;
            continue;
        }
        let line = back_edge_polyline(topology, arrangement, grid, index, contour);
        if let Some(other) = lines.iter().enumerate().position(|(other, points)| {
            let (shared, meet) = meetings(back, &line, ends(topology, other), points);
            !compatible(&line, points, shared, &meet)
        }) {
            blocked = format!(
                "it would cross {}",
                super::describe::connection(flow, merges, topology, other)
            );
            culprit = Some(other);
            continue;
        }
        if !drawn
            .iter()
            .all(|earlier| compatible(&line, earlier, false, &[]))
        {
            "it would cross the iteration back edge of a nested cycle".clone_into(&mut blocked);
            culprit = None;
            continue;
        }
        return Ok((contour, line));
    }
    Err((blocked, culprit))
}

/// Chooses a contour for every loop, innermost first, or reports that one of
/// them has none on the side it was given.
///
/// A contour is searched outward from the body's own edge, lane by lane.
/// Moving out lengthens the two horizontal runs, so a nearer lane is
/// preferred.
pub(super) fn contours(
    flow: &Flow,
    merges: &[WireMerge],
    topology: &Topology,
    arrangement: &Arrangement,
    sides: &[Side],
) -> Result<Vec<Contour>, super::Obstruction> {
    let grid = Grid::of(topology, arrangement);
    let lines = (0..topology.connections.len())
        .map(|index| polyline(topology, arrangement, &grid, index))
        .collect::<Vec<_>>();
    let mut chosen: Vec<Option<Contour>> = vec![None; topology.loops.len()];
    let mut drawn: Vec<Vec<Point>> = Vec::new();
    // Innermost first: a nested back edge becomes part of what the enclosing one
    // has to clear, and `topology.loops` runs outermost first.
    for index in (0..topology.loops.len()).rev() {
        let header = topology.loops[index].header;
        let side = sides[index];
        let body = body_columns(flow, topology, arrangement, header);
        let nested = nested_back_edges(flow, topology, &grid, header, &chosen);
        let (contour, line) = climb(
            flow,
            merges,
            topology,
            arrangement,
            &grid,
            &lines,
            &drawn,
            &body,
            &nested,
            index,
            side,
        )
        .map_err(|(blocked, culprit)| {
            let side = match side {
                Side::Left => "left",
                Side::Right => "right",
            };
            super::Obstruction {
                loop_index: Some(index),
                connection: culprit,
                span: flow.blocks[header].span,
                message: format!(
                    "the iteration back edge of {} cannot climb the {side} of its body: {blocked}. Reorder the branches so the routes that repeat the body sit at one edge",
                    super::describe::loop_name(flow, header)
                ),
            }
        })?;
        chosen[index] = Some(contour);
        drawn.push(line);
    }

    Ok(chosen.into_iter().flatten().collect())
}
