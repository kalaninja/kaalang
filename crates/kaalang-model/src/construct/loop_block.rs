//! Decides the contour of every loop return.
//!
//! RFC 0002 §8 sends a return upward outside its body and horizontally into its
//! entry junction, and RFC 0002 §7 keeps a break's first wire merge, iteration
//! tail, or end below the body it leaves. A contour therefore needs a column
//! outside the body that no route occupies over the return's whole rank span,
//! and two horizontal runs that meet nothing.
//!
//! A nearer column is preferred, because moving out lengthens the two
//! horizontal runs, but it is not the only one worth trying: a run that crosses
//! the near gap leaves a further column as the only clear place. The search
//! therefore steps outward, column by column, to the far edge of the diagram.
//!
//! RFC 0002 §8 only *prefers* a side, so a blocked preferred side never decides
//! realizability: the caller hands this module the side to try and flips it
//! when no contour holds.

use std::collections::BTreeSet;

use crate::model::{Flow, WireMerge};
use crate::topology::{Destination, NodeId, Source, Topology, Vertex};

use crate::geometry::{Point, compatible};

use super::verify::{Grid, ends, meetings, polyline, return_polyline};
use super::{Arrangement, Contour, Side};

/// The columns one loop's body occupies: the vertices of its own blocks and
/// their cases, together with its entry and tail.
pub(super) fn body_columns(
    flow: &Flow,
    topology: &Topology,
    arrangement: &Arrangement,
    header: usize,
    nested: &[Option<Contour>],
) -> BTreeSet<i32> {
    let end = flow.blocks[header].loop_end.expect("a loop owns a body");
    let body = header + 1..end;
    let mut columns = BTreeSet::new();
    for node in &topology.nodes {
        let block = match node.id {
            NodeId::Block(block) => block,
            NodeId::Case { choice, .. } => choice,
            NodeId::Start => continue,
        };
        if body.contains(&block) {
            columns.insert(arrangement.column[&Vertex::Node(node.id)]);
        }
    }
    for (index, loop_) in topology.loops.iter().enumerate() {
        if loop_.header == header || body.contains(&loop_.header) {
            columns.insert(arrangement.column[&Vertex::Junction(loop_.entry)]);
            columns.insert(arrangement.column[&Vertex::Junction(loop_.tail)]);
        }
        // A nested return already drawn beside the body widens what an
        // enclosing contour has to clear, so the enclosing edge moves past the
        // column that return climbs beside.
        if body.contains(&loop_.header)
            && let Some(contour) = nested.get(index).copied().flatten()
        {
            columns.insert(match contour.side {
                Side::Left => contour.column - 1,
                Side::Right => contour.column + 1,
            });
        }
    }
    columns
}

/// Chooses a contour for every loop, innermost first, or reports that one of
/// them has none on the side it was given.
///
/// A contour is searched outward from the body's own edge: first the lanes in
/// the gap beside it, then the lanes beyond the next column, and so on to the
/// far edge of the diagram. Moving out lengthens the two horizontal runs, so a
/// nearer lane is preferred, but a further one can still be the only clear
/// place when a run crosses the near gap.
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
    // Beyond the widest column nothing can be in a vertical's way, so the
    // search ends there.
    let reach = arrangement
        .column
        .values()
        .copied()
        .chain(arrangement.routes.iter().flat_map(|route| {
            [route.departure, route.arrival]
                .into_iter()
                .chain(route.runs.iter().flat_map(|run| [run.enter, run.exit]))
        }))
        .fold((0, 0), |(low, high), column| {
            (low.min(column), high.max(column))
        });

    // Innermost first: a nested return becomes part of what the enclosing one
    // has to clear, and `topology.loops` runs outermost first.
    for index in (0..topology.loops.len()).rev() {
        let loop_ = topology.loops[index];
        let side = sides[index];
        let body = body_columns(flow, topology, arrangement, loop_.header, &chosen);
        let edge = match side {
            Side::Left => body.iter().min(),
            Side::Right => body.iter().max(),
        }
        .copied()
        .expect("a repeating loop owns an entry and a tail");
        let span = match side {
            Side::Left => edge - reach.0,
            Side::Right => reach.1 - edge,
        }
        .max(0)
            + 2;
        let back = (
            Source::Junction(loop_.tail),
            Destination::Junction(loop_.entry),
        );
        let mut blocked = String::new();
        let mut culprit = None;
        let mut placed = None;
        'search: for step in 0..span {
            for lane in 0..super::verify::contour_lanes() {
                let column = match side {
                    Side::Left => edge - step,
                    Side::Right => edge + step,
                };
                let contour = Contour { side, column, lane };
                let line = return_polyline(
                    topology,
                    arrangement,
                    &grid,
                    loop_.tail,
                    loop_.entry,
                    contour,
                );
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
                    "it would cross the return of a loop nested inside it".clone_into(&mut blocked);
                    culprit = None;
                    continue;
                }
                placed = Some((contour, line));
                break 'search;
            }
        }
        let Some((contour, line)) = placed else {
            let side = match side {
                Side::Left => "left",
                Side::Right => "right",
            };
            return Err(super::Obstruction {
                loop_index: Some(index),
                connection: culprit,
                span: flow.blocks[loop_.header].span,
                message: format!(
                    "the return of {} cannot climb the {side} of its body: {blocked}. Reorder the branches so the routes that repeat the body sit at one edge",
                    super::describe::loop_name(flow, loop_.header)
                ),
            });
        };
        chosen[index] = Some(contour);
        drawn.push(line);
    }

    Ok(chosen.into_iter().flatten().collect())
}
