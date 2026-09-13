//! Decides the contour of every loop return.
//!
//! RFC 0002 §8 sends a return upward outside its body and horizontally into its
//! entry junction, and RFC 0002 §7 keeps a break's first wire merge, iteration
//! tail, or end below the body it leaves. A contour therefore needs a column
//! outside the body that no route occupies over the return's whole rank span,
//! and two horizontal runs that meet nothing.
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

use super::verify::{Grid, ends, meetings, polyline, return_polyline};
use super::{Arrangement, Contour, Side};

/// The columns one loop's whole body occupies, independently of its ranks.
/// Moving a body vertex below the tail does not remove it from the body, and
/// shortening a return does not shrink the extent it has to clear.
///
/// A return nested inside this body is not one of them. It climbs beside the
/// body in a lane of its own, and an enclosing return takes the next lane out
/// rather than a whole column. `nested_returns` is what holds the two apart:
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
            // Entries and tails belong to their own lexical loop. Those of
            // this body were inserted above; reaching an enclosing tail or a
            // following loop does not make that continuation part of the body.
            if topology
                .loops
                .iter()
                .any(|loop_| junction == loop_.entry || junction == loop_.tail)
            {
                continue;
            }
            let mut arrivals = topology.incoming(vertex).peekable();
            if arrivals.peek().is_some()
                && arrivals.all(|edge| vertices.contains(&Vertex::from(edge.source)))
            {
                vertices.insert(vertex);
                settled = false;
            }
        }
    }
    vertices
}

/// Where the returns nested inside one body already climb.
///
/// An enclosing return has to stay outside those too, and a crossing check
/// cannot see one whose rows do not overlap its own: the two returns may share
/// no row at all and still leave the enclosing one inside the body it is
/// supposed to clear. The position carries the nested return's lane, so the
/// enclosing one only needs the next lane out, not a column of its own.
pub(super) fn nested_returns(
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
/// column of the body, and every return nested inside it (RFC 0002 §8).
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

/// Where one return ended up, or what stopped the nearest candidate.
struct Climb {
    placed: Option<(Contour, Vec<Point>)>,
    blocked: String,
    /// The connection to blame, when one is, so the caller can give it more
    /// room and try again.
    culprit: Option<usize>,
}

/// What one return has to clear, and where it may climb.
struct Search<'a> {
    grid: &'a Grid,
    lines: &'a [Vec<Point>],
    drawn: &'a [Vec<Point>],
    body: &'a BTreeSet<i32>,
    nested: &'a [i32],
    back: (Source, Destination),
    side: Side,
    edge: i32,
}

impl Search<'_> {
    /// The nearest contour this return can climb, or why the nearest candidate
    /// failed.
    ///
    /// The candidates are the lanes beside the body's own edge column, nearest
    /// first, and nothing beyond them. A lane is the whole of a contour's
    /// position, which is what a presentation can realize: it measures the
    /// body's boxes and steps that many lanes clear of them, having no column
    /// of its own to put a rail in. A topology never needs more lanes than it
    /// has loops, because only a return climbs there (RFC 0003 §2.4).
    fn climb(
        &self,
        flow: &Flow,
        merges: &[WireMerge],
        topology: &Topology,
        arrangement: &Arrangement,
        loop_: crate::topology::Loop,
    ) -> Climb {
        let mut blocked = String::new();
        let mut culprit = None;
        for lane in 0..super::verify::contour_lanes(topology) {
            let contour = Contour {
                side: self.side,
                column: self.edge,
                lane,
            };
            if !outside(
                self.grid,
                self.side,
                self.grid.contour(contour),
                self.body,
                self.nested,
            ) {
                "it would climb inside the body it leaves".clone_into(&mut blocked);
                culprit = None;
                continue;
            }
            let line = return_polyline(
                topology,
                arrangement,
                self.grid,
                loop_.tail,
                loop_.entry,
                contour,
            );
            if let Some(other) = self.lines.iter().enumerate().position(|(other, points)| {
                let (shared, meet) = meetings(self.back, &line, ends(topology, other), points);
                !compatible(&line, points, shared, &meet)
            }) {
                blocked = format!(
                    "it would cross {}",
                    super::describe::connection(flow, merges, topology, other)
                );
                culprit = Some(other);
                continue;
            }
            if !self
                .drawn
                .iter()
                .all(|earlier| compatible(&line, earlier, false, &[]))
            {
                "it would cross the return of a loop nested inside it".clone_into(&mut blocked);
                culprit = None;
                continue;
            }
            return Climb {
                placed: Some((contour, line)),
                blocked,
                culprit,
            };
        }
        Climb {
            placed: None,
            blocked,
            culprit,
        }
    }
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
    // Innermost first: a nested return becomes part of what the enclosing one
    // has to clear, and `topology.loops` runs outermost first.
    for index in (0..topology.loops.len()).rev() {
        let loop_ = topology.loops[index];
        let side = sides[index];
        let body = body_columns(flow, topology, arrangement, loop_.header);
        let nested = nested_returns(flow, topology, &grid, loop_.header, &chosen);
        let edge = match side {
            Side::Left => body.iter().min(),
            Side::Right => body.iter().max(),
        }
        .copied()
        .expect("a repeating loop owns an entry and a tail");
        let back = (
            Source::Junction(loop_.tail),
            Destination::Junction(loop_.entry),
        );
        let search = Search {
            grid: &grid,
            lines: &lines,
            drawn: &drawn,
            body: &body,
            nested: &nested,
            back,
            side,
            edge,
        };
        let Climb {
            placed,
            blocked,
            culprit,
        } = search.climb(flow, merges, topology, arrangement, loop_);
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
