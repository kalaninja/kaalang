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
use crate::topology::{Destination, NodeId, Source, Topology, Vertex};

use crate::geometry::{Point, compatible};

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

/// Compaction must leave room for each nested frame, not just its individual
/// vertices and routes. A tail may enter that envelope after either its row
/// rises or its column joins one occupied farther down in the nested body.
pub(super) fn clears_nested_boundaries(
    flow: &Flow,
    topology: &Topology,
    arrangement: &Arrangement,
) -> bool {
    if topology.loop_boundaries.len() < 2 {
        return true;
    }
    let grid = Grid::of(topology, arrangement);
    let lines = (0..topology.connections.len())
        .map(|index| polyline(topology, arrangement, &grid, index))
        .collect::<Vec<_>>();
    let backs = (0..topology.loops.len())
        .map(|index| {
            back_edge_polyline(
                topology,
                arrangement,
                &grid,
                index,
                arrangement.contours[index],
            )
        })
        .collect::<Vec<_>>();
    for boundary in &topology.loop_boundaries {
        let outers = topology.loops.iter().enumerate().filter(|(_, loop_)| {
            let end = flow.blocks[loop_.header]
                .loop_end
                .expect("a loop owns a body");
            (loop_.header + 1..end).contains(&boundary.header)
        });
        if outers.clone().next().is_none() {
            continue;
        }
        let mut body = body_vertices(flow, topology, boundary.header);
        body.extend(boundary.result.map(Vertex::from));
        let mut points = body
            .iter()
            .map(|vertex| Point {
                x: grid.column(arrangement.column[vertex]),
                y: grid.rank(arrangement.rank[vertex]),
            })
            .collect::<Vec<_>>();
        points.extend(
            topology
                .connections
                .iter()
                .zip(&lines)
                .filter(|(edge, _)| {
                    body.contains(&Vertex::from(edge.source)) && body.contains(&edge.destination)
                })
                .flat_map(|(_, line)| line.iter().copied()),
        );
        points.extend(
            topology
                .loops
                .iter()
                .zip(&backs)
                .filter(|(loop_, _)| (boundary.header..boundary.end).contains(&loop_.header))
                .flat_map(|(_, line)| line.iter().copied()),
        );
        let first = points.first().expect("a cycle body includes its entry");
        let (left, top, right, bottom) = points.iter().fold(
            (first.x, first.y, first.x, first.y),
            |(left, top, right, bottom), point| {
                (
                    left.min(point.x),
                    top.min(point.y),
                    right.max(point.x),
                    bottom.max(point.y),
                )
            },
        );
        if outers.into_iter().any(|(index, _)| {
            backs[index].windows(2).any(|segment| {
                segment[0].x.min(segment[1].x) <= right
                    && segment[0].x.max(segment[1].x) >= left
                    && segment[0].y.min(segment[1].y) <= bottom
                    && segment[0].y.max(segment[1].y) >= top
            })
        }) {
            return false;
        }
    }
    true
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
