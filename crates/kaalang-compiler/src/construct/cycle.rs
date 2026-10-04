//! Measures cycle bodies and chooses back-edge contours.
//! The preferred search tries lanes beside the body on the caller's chosen side,
//! nearest first. Failure leaves other sides and the complete sweep available.

use std::collections::BTreeSet;

use super::{
    Arrangement, Contour, Side,
    verify::{Grid, back_edge_polyline, ends, meetings},
};
use crate::{
    geometry::{Point, compatible},
    model::{Flow, WireMerge},
    topology::{Destination, Source, Topology, Vertex},
};

/// Body columns, independent of ranks: vertices below the tail still count.
/// Nested back edges occupy lanes, checked separately by `nested_back_edges`.
pub(super) fn body_columns(
    flow: &Flow,
    topology: &Topology,
    arrangement: &Arrangement,
    header: usize,
) -> BTreeSet<i32> {
    topology
        .body_vertices(flow, header)
        .into_iter()
        .map(|vertex| arrangement.column[&vertex])
        .collect()
}

/// Nested back-edge positions, including their lanes. The enclosing contour
/// must clear them even when their row spans do not overlap.
pub(super) fn nested_back_edges(
    flow: &Flow,
    topology: &Topology,
    grid: &Grid,
    header: usize,
    chosen: &[Option<Contour>],
) -> Vec<i32> {
    let end = flow.blocks[header].cycle_end.expect("a cycle owns a body");
    topology
        .cycles
        .iter()
        .enumerate()
        .filter(|(_, cycle)| (header + 1..end).contains(&cycle.header))
        .filter_map(|(index, _)| chosen.get(index).copied().flatten())
        .map(|contour| grid.contour(contour))
        .collect()
}

/// Whether one contour climbs outside everything its body occupies: every
/// column of the body, and every back edge nested inside it.
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

/// Tries lanes beside `edge`, the body's outermost column on `side`, nearest
/// first. On failure, reports the nearest candidate's obstruction and any
/// blocking connection.
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
    edge: i32,
) -> Result<(Contour, Vec<Point>), (String, Option<usize>)> {
    let cycle = topology.cycles[index];
    let back = (
        Source::Junction(cycle.tail),
        Destination::Junction(cycle.entry),
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

/// Chooses a contour for every cycle, innermost first, or reports that one of
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
    let (grid, lines) = super::verify::drawing(topology, arrangement);
    let mut chosen: Vec<Option<Contour>> = vec![None; topology.cycles.len()];
    let mut drawn: Vec<Vec<Point>> = Vec::new();
    // Innermost first: a nested back edge becomes part of what the enclosing one
    // has to clear, and `topology.cycles` runs outermost first.
    for index in (0..topology.cycles.len()).rev() {
        let header = topology.cycles[index].header;
        let side = sides[index];
        let body = body_columns(flow, topology, arrangement, header);
        let nested = nested_back_edges(flow, topology, &grid, header, &chosen);
        let outermost = |columns: &BTreeSet<i32>| {
            match side {
                Side::Left => columns.first(),
                Side::Right => columns.last(),
            }
            .copied()
            .expect("a repeating cycle owns an entry and a tail")
        };
        let from = |edge| {
            climb(
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
                edge,
            )
        };
        let climbed = from(outermost(&body)).or_else(|blocked| {
            // A route between two body vertices may run beyond every one of
            // them, down a branch column, where every lane nearer the body
            // crosses it.
            let routed = routed_columns(flow, topology, arrangement, header, &body);
            if outermost(&routed) == outermost(&body) {
                return Err(blocked);
            }
            from(outermost(&routed)).map_err(|_| blocked)
        });
        let (contour, line) = climbed.map_err(|(blocked, culprit)| {
            let side = match side {
                Side::Left => "left",
                Side::Right => "right",
            };
            super::Obstruction {
                cycle_index: Some(index),
                connection: culprit,
                span: flow.blocks[header].span,
                message: format!(
                    "the iteration back edge of {} cannot climb the {side} of its body: {blocked}. Reorder the branches so the routes that repeat the body sit at one edge",
                    super::describe::cycle_name(flow, header)
                ),
            }
        })?;
        chosen[index] = Some(contour);
        drawn.push(line);
    }

    Ok(chosen.into_iter().flatten().collect())
}

/// The body columns together with every column a route between two body
/// vertices occupies.
fn routed_columns(
    flow: &Flow,
    topology: &Topology,
    arrangement: &Arrangement,
    header: usize,
    body: &BTreeSet<i32>,
) -> BTreeSet<i32> {
    let vertices = topology.body_vertices(flow, header);
    let mut columns = body.clone();
    for (connection, route) in topology.connections.iter().zip(&arrangement.routes) {
        if vertices.contains(&Vertex::from(connection.source))
            && vertices.contains(&connection.destination)
        {
            columns.extend([route.departure, route.arrival]);
            columns.extend(route.runs.iter().flat_map(|run| [run.enter, run.exit]));
        }
    }
    columns
}
