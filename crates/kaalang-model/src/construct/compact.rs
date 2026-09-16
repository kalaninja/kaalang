//! Optional simplification of an already checked diagram witness.

use super::{Arrangement, Run, RunLine, Side, sweep::compress};
use crate::model::Flow;
use crate::topology::{Topology, Vertex};

/// Every replacement passes the complete verifier after normalization and keeps
/// back edges outside nested body envelopes. A failed candidate leaves the
/// original witness available. Ranks only rise. With ranks fixed, contours only
/// approach their bodies, and coordinates,
/// lanes and runs only disappear; these finite measures make the iteration terminate.
///
/// ponytail: local simplifications, not a global minimum of bends or area;
/// extend the candidates only for a concrete remaining readability defect.
pub(crate) fn arrangement(flow: &Flow, topology: &Topology, built: &mut Arrangement) {
    loop {
        let mut changed = contours(flow, topology, built);
        changed |= shortcuts(flow, topology, built);
        changed |= lanes(flow, topology, built);
        changed |= columns(flow, topology, built);
        changed |= lift(flow, topology, built);
        changed |= rows(flow, topology, built);
        if !changed {
            break;
        }
    }
}

/// Join adjacent horizontal lanes when their routes can share one rail.
fn lanes(flow: &Flow, topology: &Topology, built: &mut Arrangement) -> bool {
    let mut changed = false;
    for gap in 0..built.gap_lanes.len() {
        for lane in (1..built.gap_lanes[gap]).rev() {
            if lane >= built.gap_lanes[gap] {
                continue;
            }
            let mut candidate = built.clone();
            candidate.gap_lanes[gap] -= 1;
            for route in candidate.all_routes_mut() {
                for run in &mut route.runs {
                    if let RunLine::Lane {
                        gap: at,
                        lane: level,
                    } = &mut run.line
                    {
                        *level -= usize::from(*at == gap && *level >= lane);
                    }
                }
            }
            if keep(flow, topology, built, candidate) {
                changed = true;
            }
        }
    }
    changed
}

/// Bring a straight back edge to the edge of its whole body when the gap is free.
fn contours(flow: &Flow, topology: &Topology, built: &mut Arrangement) -> bool {
    let mut changed = false;
    if built.back_routes.is_empty() {
        for (index, loop_) in topology.loops.iter().enumerate().rev() {
            let body = super::body_vertices(flow, topology, loop_.header);
            let columns = body.iter().map(|vertex| built.column[vertex]);
            let column = match built.contours[index].side {
                Side::Left => columns.min(),
                Side::Right => columns.max(),
            }
            .expect("a cycle body includes its entry and tail");
            if column == built.contours[index].column {
                continue;
            }
            for lane in 0..topology.loops.len() {
                let mut candidate = built.clone();
                candidate.contours[index].column = column;
                candidate.contours[index].lane = lane;
                if keep(flow, topology, built, candidate) {
                    changed = true;
                    break;
                }
            }
        }
    }
    changed
}

/// Replace successive sideways runs with one of their direct shortcuts.
fn shortcuts(flow: &Flow, topology: &Topology, built: &mut Arrangement) -> bool {
    let mut changed = false;
    for wire in 0..built.routes.len() {
        'shortcut: for first in 0..built.routes[wire].runs.len() {
            for last in (first + 1..built.routes[wire].runs.len()).rev() {
                for at in [first, last] {
                    let runs = &built.routes[wire].runs;
                    let mut run = runs[at];
                    run.enter = runs[first].enter;
                    run.exit = runs[last].exit;
                    let mut candidate = built.clone();
                    candidate.routes[wire]
                        .runs
                        .splice(first..=last, (run.enter != run.exit).then_some(run));
                    if keep(flow, topology, built, candidate) {
                        changed = true;
                        break 'shortcut;
                    }
                }
            }
        }
    }
    changed
}

/// Merge adjacent coordinates only when the full column and crossing rules permit it.
fn columns(flow: &Flow, topology: &Topology, built: &mut Arrangement) -> bool {
    let mut changed = false;
    let widest = built.column.values().copied().max().unwrap_or(0);
    for column in (1..=widest).rev() {
        if column > built.column.values().copied().max().unwrap_or(0) {
            continue;
        }
        let mut candidate = built.clone();
        map_columns(&mut candidate, |x| x - i32::from(x >= column));
        if keep(flow, topology, built, candidate) {
            changed = true;
        }
    }
    changed
}

/// Lift one vertex and its arrivals; a shared fan-out may reuse an existing lane.
fn lift(flow: &Flow, topology: &Topology, built: &mut Arrangement) -> bool {
    let mut changed = false;
    for &vertex in &topology.vertices {
        // No forward or placement relation may rise. The verifier still decides
        // which relations may share a row, without constructing earlier candidates.
        let first = topology
            .incoming(vertex)
            .chain(
                topology
                    .order
                    .iter()
                    .filter(|edge| edge.destination == vertex),
            )
            .map(|edge| built.rank[&Vertex::from(edge.source)])
            .max()
            .unwrap_or(1)
            .max(1);
        'earlier: for rank in first..built.rank[&vertex] {
            let gap = rank - 1;
            for lane in 0..=built.gap_lanes[gap] {
                let mut candidate = built.clone();
                candidate.rank.insert(vertex, rank);
                for (index, _) in topology
                    .connections
                    .iter()
                    .enumerate()
                    .filter(|(_, wire)| wire.destination == vertex)
                {
                    let route = &mut candidate.routes[index];
                    route.runs.retain(|run| match run.line {
                        RunLine::Rank(at) => at < rank,
                        RunLine::Lane { gap: at, .. } => at < gap,
                    });
                    let enter = route.runs.last().map_or(route.departure, |run| run.exit);
                    if enter != route.arrival {
                        let line = if matches!(vertex, Vertex::Junction(_)) {
                            RunLine::Rank(rank)
                        } else {
                            candidate.gap_lanes[gap] = candidate.gap_lanes[gap].max(lane + 1);
                            RunLine::Lane { gap, lane }
                        };
                        route.runs.push(Run {
                            line,
                            enter,
                            exit: route.arrival,
                        });
                    }
                }
                if keep(flow, topology, built, candidate) {
                    changed = true;
                    break 'earlier;
                }
            }
        }
    }
    changed
}

/// Fold consecutive ranks, preserving the order of their routing lanes.
fn rows(flow: &Flow, topology: &Topology, built: &mut Arrangement) -> bool {
    let mut changed = false;
    for rank in (2..built.ranks).rev() {
        let mut candidate = built.clone();
        for row in candidate.rank.values_mut() {
            *row -= usize::from(*row >= rank);
        }
        let lanes = candidate.gap_lanes[rank - 2];
        for route in candidate.all_routes_mut() {
            for run in &mut route.runs {
                match &mut run.line {
                    RunLine::Rank(row) => *row -= usize::from(*row >= rank),
                    RunLine::Lane { gap, lane } => {
                        if *gap == rank - 1 {
                            *lane += lanes;
                        }
                        *gap -= usize::from(*gap >= rank - 1);
                    }
                }
            }
        }
        candidate.gap_lanes[rank - 2] += candidate.gap_lanes.remove(rank - 1);
        candidate.ranks -= 1;
        if keep(flow, topology, built, candidate) {
            changed = true;
        }
    }
    changed
}

fn keep(
    flow: &Flow,
    topology: &Topology,
    built: &mut Arrangement,
    mut candidate: Arrangement,
) -> bool {
    compress(&mut candidate);
    if super::verify::arrangement(flow, topology, &candidate).is_err()
        || !super::loop_block::clears_nested_boundaries(flow, topology, &candidate)
    {
        return false;
    }
    *built = candidate;
    true
}

fn map_columns(built: &mut Arrangement, map: impl Fn(i32) -> i32) {
    for (exit, offset) in &mut built.exit_offset {
        let column = built.column[&Vertex::Node(exit.node)];
        *offset = map(column + *offset) - map(column);
    }
    for column in built.column.values_mut() {
        *column = map(*column);
    }
    for contour in &mut built.contours {
        contour.column = map(contour.column);
    }
    for route in built.all_routes_mut() {
        route.departure = map(route.departure);
        route.arrival = map(route.arrival);
        for run in &mut route.runs {
            run.enter = map(run.enter);
            run.exit = map(run.exit);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_outer_tail_cannot_join_a_column_inside_a_nested_frame() {
        let function = crate::tests::fixture(
            include_str!("../../../kaalang/tests/loop/behavior/nested_side_returns.rs"),
            "nested_side_returns",
        );
        let mut model = crate::build(&function).unwrap();
        model.compact_arrangement();
        let tail = Vertex::Junction(model.topology.loops[0].tail);
        let column = model.arrangement.column[&tail];
        let mut candidate = model.arrangement.clone();
        map_columns(&mut candidate, |x| x - i32::from(x >= column));
        super::super::verify::arrangement(&model.flow, &model.topology, &candidate)
            .expect("individual vertices and routes still clear each other");
        assert!(
            !keep(
                &model.flow,
                &model.topology,
                &mut model.arrangement,
                candidate,
            ),
            "the outer return would cross the nested frame"
        );
    }

    #[test]
    fn unused_lanes_can_disappear_together_during_compaction() {
        let source = super::super::tests::looping(&["repeat", "repeat", "break"]);
        let mut model = crate::build(&syn::parse_str(&source).unwrap()).unwrap();
        for lanes in &mut model.arrangement.gap_lanes {
            *lanes += 4;
        }
        super::super::verify::arrangement(&model.flow, &model.topology, &model.arrangement)
            .unwrap();
        arrangement(&model.flow, &model.topology, &mut model.arrangement);
        super::super::verify::arrangement(&model.flow, &model.topology, &model.arrangement)
            .unwrap();
    }
}
