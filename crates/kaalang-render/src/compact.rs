//! Arrangement compaction shared by kaalang renderers.

use kaalang_compiler::{
    Arrangement, Run, RunLine, SemanticModel, Side,
    topology::{Source, Topology, Vertex},
};

use crate::ArrangementVerifier;

/// Compacts `model.arrangement` in place, keeping each normalized replacement
/// only after the verifier accepts it, so a failure preserves the existing
/// witness. Each of `model.stages` needs its own call.
/// Terminates as ranks rise, then contours approach their bodies and coordinates,
/// lanes, and runs disappear.
///
/// Reaches a deterministic local fixed point, not a global minimum of bends or
/// area, as specified by RFC 0003 §2.3.
pub fn compact_arrangement(model: &mut SemanticModel) {
    if model.topology.cycles.is_empty()
        && !model
            .arrangement
            .routes
            .iter()
            .any(|route| route.runs.len() > 2)
        && steps_below(&model.topology, &model.arrangement)
            .next()
            .is_none()
    {
        return;
    }
    let verifier = ArrangementVerifier::new(&model.analysis.flow, &model.topology);
    arrangement(&verifier, &model.topology, &mut model.arrangement);
}

fn arrangement(verifier: &ArrangementVerifier<'_>, topology: &Topology, built: &mut Arrangement) {
    loop {
        let mut changed = contours(verifier, topology, built);
        changed |= shortcuts(verifier, built);
        changed |= departures(verifier, topology, built);
        changed |= lanes(verifier, built);
        changed |= columns(verifier, built);
        changed |= lift(verifier, topology, built);
        changed |= rows(verifier, built);
        if !changed {
            break;
        }
    }
}

/// Join adjacent horizontal lanes when their routes can share one rail.
fn lanes(verifier: &ArrangementVerifier<'_>, built: &mut Arrangement) -> bool {
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
            changed = keep(verifier, built, candidate) || changed;
        }
    }
    changed
}

/// Bring a straight back edge to the edge of its whole body when the gap is free.
fn contours(
    verifier: &ArrangementVerifier<'_>,
    topology: &Topology,
    built: &mut Arrangement,
) -> bool {
    let mut changed = false;
    if built.back_routes.is_empty() {
        for (index, cycle) in topology.cycles.iter().enumerate().rev() {
            let body = verifier.body_vertices(cycle.header);
            let columns = body.iter().map(|vertex| built.column[vertex]);
            let column = match built.contours[index].side {
                Side::Left => columns.min(),
                Side::Right => columns.max(),
            }
            .expect("a cycle body includes its entry and tail");
            if column == built.contours[index].column {
                continue;
            }
            for lane in 0..topology.cycles.len() {
                let mut candidate = built.clone();
                candidate.contours[index].column = column;
                candidate.contours[index].lane = lane;
                if keep(verifier, built, candidate) {
                    changed = true;
                    break;
                }
            }
        }
    }
    changed
}

/// Replace successive sideways runs with one of their direct shortcuts.
fn shortcuts(verifier: &ArrangementVerifier<'_>, built: &mut Arrangement) -> bool {
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
                    if keep(verifier, built, candidate) {
                        changed = true;
                        break 'shortcut;
                    }
                }
            }
        }
    }
    changed
}

/// The routes whose first run turns, right below the brancher, into the column
/// their branch owns. Construction starts such a route at the merge column;
/// that jog is a bend the route does not need.
fn steps_below<'a>(
    topology: &'a Topology,
    built: &'a Arrangement,
) -> impl Iterator<Item = usize> + 'a {
    topology
        .connections
        .iter()
        .enumerate()
        .filter_map(|(index, wire)| {
            let Source::Exit(exit) = wire.source else {
                return None;
            };
            let first = built.routes[index].runs.first()?;
            let RunLine::Lane { gap, .. } = first.line else {
                return None;
            };
            (gap == built.rank[&Vertex::Node(exit.node)] && first.exit == built.exit_column(exit))
                .then_some(index)
        })
}

/// Leave a branch in the column its route turned into right below the
/// brancher, so the route departs straight into its descent.
fn departures(
    verifier: &ArrangementVerifier<'_>,
    topology: &Topology,
    built: &mut Arrangement,
) -> bool {
    let mut changed = false;
    for index in steps_below(topology, built).collect::<Vec<_>>() {
        let mut candidate = built.clone();
        let route = &mut candidate.routes[index];
        route.departure = route.runs.remove(0).exit;
        changed = keep(verifier, built, candidate) || changed;
    }
    changed
}

/// Merge adjacent coordinates only when the full column and crossing rules permit it.
fn columns(verifier: &ArrangementVerifier<'_>, built: &mut Arrangement) -> bool {
    let mut changed = false;
    let widest = built.column.values().copied().max().unwrap_or(0);
    for column in (1..=widest).rev() {
        if column > built.column.values().copied().max().unwrap_or(0) {
            continue;
        }
        let mut candidate = built.clone();
        map_columns(&mut candidate, |x| x - i32::from(x >= column));
        changed = keep(verifier, built, candidate) || changed;
    }
    changed
}

/// Lift one vertex and its arrivals; a shared fan-out may reuse an existing lane.
fn lift(verifier: &ArrangementVerifier<'_>, topology: &Topology, built: &mut Arrangement) -> bool {
    let mut changed = false;
    for &vertex in &topology.vertices {
        // No forward relation may rise, and no placement relation either —
        // except the one that only orders a cycle's completion after its body.
        // The boundary rule decides where that may stand, and `keep` checks it,
        // so the candidates above the body are worth constructing.
        let first = topology
            .incoming(vertex)
            .chain(topology.order.iter().filter(|edge| {
                edge.destination == vertex && !verifier.may_rise_beside(built, edge)
            }))
            .map(|edge| built.rank[&Vertex::from(edge.source)])
            .fold(1, usize::max);
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
                if keep(verifier, built, candidate) {
                    changed = true;
                    break 'earlier;
                }
            }
        }
    }
    changed
}

/// Fold consecutive ranks, preserving the order of their routing lanes.
fn rows(verifier: &ArrangementVerifier<'_>, built: &mut Arrangement) -> bool {
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
        changed = keep(verifier, built, candidate) || changed;
    }
    changed
}

fn keep(
    verifier: &ArrangementVerifier<'_>,
    built: &mut Arrangement,
    candidate: Arrangement,
) -> bool {
    let Ok(candidate) = verifier.normalize(candidate) else {
        return false;
    };
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
    use kaalang_compiler::topology::NodeKind;
    use kaalang_testing::corpus::flow_named as fixture;

    use super::*;

    /// The flow declared by `crates/kaalang/tests/<dir>/<stem>.rs`, named after the file.
    macro_rules! fixture {
        ($dir:literal, $stem:literal) => {
            fixture(
                include_str!(concat!("../../kaalang/tests/", $dir, "/", $stem, ".rs")),
                $stem,
            )
        };
    }

    fn transitions(topology: &Topology) -> Vec<Vertex> {
        topology
            .nodes
            .iter()
            .filter(|node| node.kind == NodeKind::StageTransition)
            .map(|node| Vertex::Node(node.id))
            .collect()
    }

    #[test]
    fn stage_transitions_keep_their_final_row_through_compaction() {
        let mut model =
            kaalang_compiler::build(&fixture!("gallery/sorting", "quick_sort")).unwrap();
        let mut stages = std::mem::take(&mut model.stages);
        for part in std::iter::once(&mut model).chain(stages.iter_mut()) {
            compact_arrangement(part);
            let transitions = transitions(&part.topology);
            let Some(first) = transitions.first() else {
                continue;
            };
            let row = part.arrangement.rank[first];
            assert!(
                transitions
                    .iter()
                    .all(|transition| part.arrangement.rank[transition] == row)
            );
            assert!(
                part.topology
                    .vertices
                    .iter()
                    .filter(|vertex| !transitions.contains(vertex))
                    .all(|vertex| part.arrangement.rank[vertex] < row)
            );
            let mut broken = part.arrangement.clone();
            *broken.rank.get_mut(first).unwrap() -= 1;
            assert!(
                ArrangementVerifier::new(&part.analysis.flow, &part.topology)
                    .normalize(broken)
                    .is_err()
            );
        }
    }

    #[test]
    fn a_stage_with_a_cycle_compacts_both_transition_routes() {
        let function = fixture!("stage/behavior", "stage_cycle_alternative_outputs");
        let mut model = kaalang_compiler::build(&function).unwrap();
        let stage = &mut model.stages[0];
        assert!(!stage.topology.cycles.is_empty());
        let transitions = transitions(&stage.topology);
        assert!(transitions.len() >= 2);
        compact_arrangement(stage);
        ArrangementVerifier::new(&stage.analysis.flow, &stage.topology)
            .normalize(stage.arrangement.clone())
            .unwrap();
        let row = stage.arrangement.rank[&transitions[0]];
        assert!(
            transitions
                .iter()
                .all(|node| stage.arrangement.rank[node] == row)
        );
    }

    /// The verifier decides this on its own: the boundary rule reads the
    /// rectangle each cycle draws, not just the vertices and routes in it.
    #[test]
    fn an_outer_tail_cannot_join_a_column_inside_a_nested_frame() {
        let mut model =
            kaalang_compiler::build(&fixture!("cycle/behavior", "nested_side_returns")).unwrap();
        compact_arrangement(&mut model);
        let tail = Vertex::Junction(model.topology.cycles[0].tail);
        let column = model.arrangement.column[&tail];
        let mut candidate = model.arrangement.clone();
        map_columns(&mut candidate, |x| x - i32::from(x >= column));
        let verifier = ArrangementVerifier::new(&model.analysis.flow, &model.topology);
        let original = model.arrangement.clone();
        let refused = verifier
            .normalize(candidate.clone())
            .expect_err("the outer return would cross the nested frame");
        assert!(refused.contains("cycle"), "{refused}");
        assert!(
            !keep(&verifier, &mut model.arrangement, candidate),
            "compaction refuses what the verifier refuses"
        );
        assert_eq!(model.arrangement, original);
    }

    /// A completion outside the cycle boundary may share its body's rows.
    #[test]
    fn a_completion_stands_beside_the_cycle_it_leaves() {
        for function in [
            fixture!("gallery/sorting", "bubble_sort"),
            fixture!("cycle/behavior", "collect_steps"),
        ] {
            let name = &function.sig.ident;
            let mut model = kaalang_compiler::build(&function).unwrap();
            compact_arrangement(&mut model);
            let built = &model.arrangement;
            let beside = model.topology.cycle_boundaries.iter().any(|boundary| {
                let body = model
                    .topology
                    .body_vertices(&model.analysis.flow, boundary.header);
                let rows = body.iter().map(|vertex| built.rank[vertex]);
                let columns = body.iter().map(|vertex| built.column[vertex]);
                let (top, bottom) = (rows.clone().min(), rows.max());
                let (left, right) = (columns.clone().min(), columns.max());
                model
                    .topology
                    .connections
                    .iter()
                    .filter(|edge| boundary.results.contains(&edge.source))
                    .any(|edge| {
                        let (rank, column) = (
                            built.rank[&edge.destination],
                            built.column[&edge.destination],
                        );
                        Some(rank) >= top
                            && Some(rank) <= bottom
                            && (Some(column) < left || Some(column) > right)
                    })
            });
            assert!(
                beside,
                "{name}: every completion still waits out the body it leaves"
            );
        }
    }

    #[test]
    fn a_branch_that_stepped_sideways_below_its_brancher_departs_straight() {
        let function = fixture!("wire/behavior", "early_branch_rejoins_below_a_side_branch");
        let mut model = kaalang_compiler::build(&function).unwrap();
        let stepping = steps_below(&model.topology, &model.arrangement).collect::<Vec<_>>();
        assert!(
            !stepping.is_empty(),
            "the fixture constructs a stepping departure"
        );
        let runs = stepping
            .iter()
            .map(|&index| model.arrangement.routes[index].runs.len())
            .collect::<Vec<_>>();
        compact_arrangement(&mut model);
        for (&index, &before) in stepping.iter().zip(&runs) {
            let Source::Exit(exit) = model.topology.connections[index].source else {
                unreachable!("only exits step below their brancher");
            };
            let route = &model.arrangement.routes[index];
            assert_eq!(route.departure, model.arrangement.exit_column(exit));
            assert!(route.runs.len() < before, "the step is gone");
        }
        assert_eq!(steps_below(&model.topology, &model.arrangement).count(), 0);
    }

    #[test]
    fn unused_lanes_can_disappear_together_during_compaction() {
        let source = kaalang_testing::shapes::cycle_routes(&["repeat", "repeat", "leave"]);
        let mut model = kaalang_compiler::build(&syn::parse_str(&source).unwrap()).unwrap();
        for lanes in &mut model.arrangement.gap_lanes {
            *lanes += 4;
        }
        let verifier = ArrangementVerifier::new(&model.analysis.flow, &model.topology);
        verifier.normalize(model.arrangement.clone()).unwrap();
        compact_arrangement(&mut model);
        ArrangementVerifier::new(&model.analysis.flow, &model.topology)
            .normalize(model.arrangement.clone())
            .unwrap();
    }

    #[test]
    fn compaction_is_deterministic_and_idempotent() {
        let source = kaalang_testing::shapes::cycle_routes(&["repeat", "repeat", "leave"]);
        let function: syn::ItemFn = syn::parse_str(&source).unwrap();
        let mut first = kaalang_compiler::build(&function).unwrap();
        let mut second = kaalang_compiler::build(&function).unwrap();
        compact_arrangement(&mut first);
        compact_arrangement(&mut second);
        assert_eq!(first.arrangement, second.arrangement);

        let compacted = first.arrangement.clone();
        compact_arrangement(&mut first);
        assert_eq!(first.arrangement, compacted);
    }

    #[test]
    fn junction_arrivals_keep_their_rank_through_compaction() {
        let function = fixture!("wire/behavior", "two_merges_reach_one_consumer");
        let mut model = kaalang_compiler::build(&function).unwrap();
        compact_arrangement(&mut model);
        ArrangementVerifier::new(&model.analysis.flow, &model.topology)
            .normalize(model.arrangement.clone())
            .unwrap();

        let arrivals = model
            .topology
            .connections
            .iter()
            .enumerate()
            .filter(|(_, wire)| matches!(wire.destination, Vertex::Junction(_)))
            .filter(|(index, wire)| {
                model.arrangement.routes[*index]
                    .runs
                    .last()
                    .is_some_and(|run| {
                        run.line == RunLine::Rank(model.arrangement.rank[&wire.destination])
                    })
            })
            .count();
        assert!(arrivals >= 2, "the fixture exercises sideways arrivals");
    }
}
