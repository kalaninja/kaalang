use std::collections::BTreeSet;

use super::{Arrangement, Contour, Shape, Side, place, route, verify};
use crate::model::{Flow, SemanticModel, WireMerge};
use crate::topology::{ExitId, NodeId, Source, Topology, Vertex, linked};

fn model(source: &str) -> SemanticModel {
    let function = syn::parse_str(source).expect("the flow parses");
    crate::build(&function).expect("the flow is valid")
}

fn arrangement(model: &SemanticModel) -> Arrangement {
    super::construct(&model.flow, &model.merges, &model.topology)
        .expect("the flow has a conforming arrangement")
}

/// A loop whose body selects one route per named outcome, in that order.
/// `repeat` falls through to the end of the body, `break` leaves the loop, and
/// `end` finishes the flow from inside it.
fn looping(routes: &[&str]) -> String {
    let cases = routes
        .iter()
        .enumerate()
        .map(|(index, route)| format!("            #[case(\"Case {index} {route}.\")]"))
        .collect::<Vec<_>>()
        .join("\n");
    let outputs = (0..routes.len())
        .map(|index| format!("case_{index}"))
        .collect::<Vec<_>>()
        .join(", ");
    let arms = (0..routes.len())
        .map(|index| {
            if index + 1 == routes.len() {
                "            _ => (),".to_owned()
            } else {
                format!("            {index} => (),")
            }
        })
        .collect::<Vec<_>>()
        .join("\n");
    let bodies = routes
        .iter()
        .enumerate()
        .map(|(index, route)| match *route {
            "break" => format!("        |case_{index}| break;"),
            "end" => format!(
                "        #[action(\"Finish from case {index}.\")]\n        let end = |case_{index}| 7;"
            ),
            _ => format!(
                "        #[action(\"Advance in case {index}.\")]\n        |case_{index}| ();"
            ),
        })
        .collect::<Vec<_>>()
        .join("\n");
    let after = if routes.contains(&"break") {
        "\n    #[action(\"Return the mode.\")]\n    let end = |mode| mode;"
    } else {
        ""
    };
    format!(
        "fn probe(mode: u8) -> u8 {{
    loop {{
        #[choice(\"Which route?\")]
{cases}
        let ({outputs}) = |mode| match mode {{
{arms}
        }};
{bodies}
    }}{after}
}}
"
    )
}

/// The parts the arrangement reads, so a test can rebuild a candidate.
struct Parts {
    flow: Flow,
    merges: Vec<WireMerge>,
    topology: Topology,
}

/// The analyzed parts of a semantic model, without invoking construction.
fn parts_of(source: &str) -> Option<Parts> {
    let model = crate::build(&syn::parse_str(source).ok()?).ok()?;
    Some(Parts {
        flow: model.flow,
        merges: model.merges,
        topology: model.topology,
    })
}

/// Bounded reference enumeration, independent of conflict-guided pruning. It
/// shares placement and corridor planning, so agreement tests pruning on these
/// cases; it does not prove normalization or general completeness.
///
/// Enumeration bounds: every subset of sunk iteration tails, every contour
/// side, every contour column in use and one past each end of them, every lane
/// a column gap offers, and every corridor assignment in which at most two
/// connections differ from `Direct`. The production search prunes by following
/// the conflicts its planner and contour search report; this one does not
/// prune at all inside those bounds, and shares none of that code.
fn reference(parts: &Parts) -> bool {
    let Parts {
        flow,
        merges,
        topology,
    } = parts;
    let tails = topology
        .loops
        .iter()
        .map(|loop_| Vertex::Junction(loop_.tail))
        .collect::<Vec<_>>();
    let count = topology.connections.len();
    let deviations = std::iter::once(Vec::new())
        .chain((0..count).flat_map(|one| {
            [Shape::Deferred, Shape::Aside]
                .into_iter()
                .map(move |shape| vec![(one, shape)])
        }))
        .chain((0..count).flat_map(|one| {
            (one + 1..count).flat_map(move |two| {
                [Shape::Deferred, Shape::Aside]
                    .into_iter()
                    .flat_map(move |first| {
                        [Shape::Deferred, Shape::Aside]
                            .into_iter()
                            .map(move |second| vec![(one, first), (two, second)])
                    })
            })
        }))
        .collect::<Vec<_>>();

    for mask in 0..1usize << tails.len() {
        let sunk = (0..tails.len())
            .filter(|index| mask & (1 << index) != 0)
            .map(|index| tails[index])
            .collect::<BTreeSet<_>>();
        for choice in 0..1usize << topology.loops.len() {
            let sides = (0..topology.loops.len())
                .map(|loop_| {
                    if choice & (1 << loop_) == 0 {
                        Side::Left
                    } else {
                        Side::Right
                    }
                })
                .collect::<Vec<_>>();
            let Ok(placement) = place::place(topology, flow, merges, &sunk, &sides) else {
                continue;
            };
            for deviation in &deviations {
                let mut shapes = vec![Shape::Direct; count];
                for &(index, shape) in deviation {
                    shapes[index] = shape;
                }
                let Ok(plan) = route::plan(flow, merges, topology, &placement, &shapes) else {
                    continue;
                };
                let mut candidate = super::assemble(topology, placement.clone(), &plan);
                for contours in contour_assignments(&candidate, &sides) {
                    candidate.contours = contours;
                    if verify::arrangement(flow, topology, &candidate).is_ok() {
                        return true;
                    }
                }
            }
        }
    }
    false
}

/// Every contour assignment worth trying for one candidate: each loop takes any
/// column in use or one past either end of them, in any lane, on the side the
/// placement was given.
fn contour_assignments(candidate: &Arrangement, sides: &[Side]) -> Vec<Vec<Contour>> {
    let low = candidate.column.values().copied().min().unwrap_or(0) - 1;
    let high = candidate.column.values().copied().max().unwrap_or(0) + 1;
    let lanes = 3;
    let mut assignments = vec![Vec::new()];
    for &side in sides {
        assignments = assignments
            .into_iter()
            .flat_map(|chosen| {
                (low..=high).flat_map(move |column| {
                    let chosen = chosen.clone();
                    (0..lanes).map(move |lane| {
                        let mut next = chosen.clone();
                        next.push(Contour { side, column, lane });
                        next
                    })
                })
            })
            .collect();
    }
    assignments
}

/// Every shape of a three-case loop body, and whether construction succeeds. The
/// reference procedure has to agree on each one, in both directions: a flow the
/// search accepts has a candidate the reference finds, and one it rejects has
/// none.
#[test]
fn the_reference_procedure_agrees_with_the_search() {
    let mut checked = 0;
    for first in ["repeat", "break", "end"] {
        for second in ["repeat", "break", "end"] {
            for third in ["repeat", "break", "end"] {
                let routes = [first, second, third];
                // A body with no repeating route draws no return, and one with
                // two `end` cases needs them adjacent; both are other rules'
                // business.
                let source = looping(&routes);
                // A flow refused before the arrangement has no topology to
                // arrange, and realizability never had a say in it.
                let Some(parts) = parts_of(&source) else {
                    continue;
                };
                let accepted =
                    super::construct(&parts.flow, &parts.merges, &parts.topology).is_ok();
                assert_eq!(
                    reference(&parts),
                    accepted,
                    "{routes:?}: the search and the reference procedure disagree"
                );
                checked += 1;
            }
        }
    }
    assert!(checked >= 8, "the enumeration covered too few shapes");
}

/// Semantic obstructions retain their diagnostic priority independently of
/// construction, including the specific branch-reordering advice of §4.5.
#[test]
fn semantic_restrictions_keep_their_specific_diagnostics() {
    for (routes, expected) in [
        // The `end` case on one side of the repeat splits a merge RFC 0001 §7
        // needs adjacent, and that rule speaks first.
        (["end", "repeat", "break"], "must be adjacent"),
        (
            ["break", "repeat", "break"],
            "a repeating kaalang branch cannot lie between breaks to the same loop; reorder the branches",
        ),
    ] {
        let source = looping(&routes);
        let function = syn::parse_str(&source).expect("the probe parses");
        let built = crate::build(&function);
        assert!(
            built.is_err(),
            "{routes:?} should have no conforming diagram"
        );
        let message = built.err().expect("the flow is rejected").to_string();
        assert!(
            message.contains(expected),
            "{routes:?}: expected {expected}, got {message}"
        );
    }
}

/// A blocked preferred contour never decides realizability: RFC 0002 §8 only
/// prefers a side.
#[test]
fn a_loop_takes_the_clear_contour_whatever_the_preference() {
    for (routes, side) in [
        (["break", "repeat", "repeat"], Side::Right),
        (["repeat", "repeat", "break"], Side::Left),
    ] {
        let source = looping(&routes);
        let function = syn::parse_str(&source).expect("the probe parses");
        let model = crate::build(&function).expect("the probe has a conforming diagram");
        assert_eq!(arrangement(&model).contours[0].side, side, "{routes:?}");
    }
}

/// A route that finishes the flow from inside a body blocks no contour by
/// itself: nothing forces the end node below the iteration tail.
#[test]
fn a_terminal_route_between_two_repeats_still_draws() {
    let source = looping(&["repeat", "end", "repeat"]);
    let function = syn::parse_str(&source).expect("the probe parses");
    let model = crate::build(&function).expect("the probe has a conforming diagram");
    let tail = Vertex::Junction(model.topology.loops[0].tail);
    let end = Vertex::Node(NodeId::Block(model.flow.blocks.len() - 1));
    assert!(
        arrangement(&model).rank[&tail] > arrangement(&model).rank[&end],
        "the tail sinks below end so the finishing route clears its flank"
    );
}

/// A cycle is a projection bug, not an undrawable flow: ranking refuses it and
/// the search reports it as an internal error rather than blaming the author.
#[test]
fn a_cycle_is_refused_rather_than_ranked() {
    let chain =
        place::rows(&linked(&[(0, 1)]), &[], &BTreeSet::new()).expect("a chain has an order");
    assert!(chain[&Vertex::Node(NodeId::Block(0))] < chain[&Vertex::Node(NodeId::Block(1))]);

    assert_eq!(
        place::rows(&linked(&[(0, 1), (1, 0)]), &[], &BTreeSet::new()),
        Err("the connections form a cycle, so no node can be lowest".to_owned())
    );
}

/// One named break of a valid arrangement.
type Mutation = (&'static str, Box<dyn Fn(&mut Arrangement)>);

/// Every way of breaking a checked arrangement that the plan names.
fn mutations() -> Vec<Mutation> {
    let mut mutations = coverage_mutations();
    mutations.extend(geometry_mutations());
    mutations
}

/// Breaks a arrangement states outright, before any route is drawn.
fn coverage_mutations() -> Vec<Mutation> {
    vec![
        (
            "a dropped corridor",
            Box::new(|arrangement: &mut Arrangement| {
                arrangement.routes.pop();
            }),
        ),
        (
            "a dropped contour",
            Box::new(|arrangement: &mut Arrangement| {
                arrangement.contours.pop();
            }),
        ),
        (
            "a route that does not descend",
            Box::new(|arrangement: &mut Arrangement| {
                let deepest = arrangement
                    .rank
                    .iter()
                    .max_by_key(|(_, rank)| **rank)
                    .map(|(vertex, _)| *vertex)
                    .expect("a arrangement ranks its vertices");
                arrangement.rank.insert(deepest, 0);
            }),
        ),
        (
            "a corridor that arrives away from its destination",
            Box::new(|arrangement: &mut Arrangement| {
                for route in &mut arrangement.routes {
                    route.arrival += 1;
                }
            }),
        ),
        (
            "a corridor that leaves a column its exit does not own",
            Box::new(|arrangement: &mut Arrangement| {
                for route in &mut arrangement.routes {
                    route.departure += 1;
                }
            }),
        ),
    ]
}

/// Breaks that only the drawn geometry rejects: the coverage rules above hold
/// of every one of them, so each has to reach the crossing and overlap checks.
fn geometry_mutations() -> Vec<Mutation> {
    vec![
        (
            "reversed branch order",
            Box::new(|arrangement: &mut Arrangement| {
                // The columns the branches are drawn in, not the reserved
                // widths: the check reads the placement, so the mutation has
                // to move the placement.
                let cases = arrangement
                    .column
                    .iter()
                    .filter(|(vertex, _)| matches!(vertex, Vertex::Node(NodeId::Case { .. })))
                    .map(|(vertex, column)| (*vertex, *column))
                    .collect::<Vec<_>>();
                for (index, (vertex, _)) in cases.iter().enumerate() {
                    arrangement
                        .column
                        .insert(*vertex, cases[cases.len() - 1 - index].1);
                }
            }),
        ),
        (
            "a corridor moved across another",
            Box::new(|arrangement: &mut Arrangement| {
                for route in &mut arrangement.routes {
                    for run in &mut route.runs {
                        run.enter += 1;
                    }
                }
            }),
        ),
        (
            "a contour inside the body",
            Box::new(|arrangement: &mut Arrangement| {
                if let Some(contour) = arrangement.contours.first_mut() {
                    *contour = Contour {
                        side: contour.side,
                        column: contour.column,
                        lane: 64,
                    };
                }
            }),
        ),
        (
            "unrelated routes sharing one run",
            Box::new(|arrangement: &mut Arrangement| {
                for route in &mut arrangement.routes {
                    for run in &mut route.runs {
                        run.enter = 0;
                        run.exit = 0;
                    }
                }
            }),
        ),
        (
            "a contour on the other side",
            Box::new(|arrangement: &mut Arrangement| {
                if let Some(contour) = arrangement.contours.first_mut() {
                    contour.side = match contour.side {
                        Side::Left => Side::Right,
                        Side::Right => Side::Left,
                    };
                }
            }),
        ),
        (
            "a contour climbing one column inward",
            Box::new(|arrangement: &mut Arrangement| {
                if let Some(contour) = arrangement.contours.first_mut() {
                    contour.column += match contour.side {
                        Side::Left => 1,
                        Side::Right => -1,
                    };
                }
            }),
        ),
    ]
}

/// Every mutation of a checked arrangement is caught by the verifier, which
/// is what makes a positive result a witness rather than a coincidence.
///
/// What no mutation of this probe reaches is the crossing rule itself: with one
/// lane per rank gap, every sideways move a mutation can make either keeps the
/// drawing legal or bends a route diagonally first. Those rules are exercised
/// from both ends instead — by the impossibility tests here, which pass only
/// because every candidate arrangement is rejected for crossing, and by the
/// renderer's own tests over `geometry`, the one implementation both checks
/// call.
#[test]
fn the_verifier_rejects_every_mutation() {
    let model = model(&looping(&["repeat", "repeat", "break"]));
    let flow = &model.flow;
    let topology = &model.topology;
    let valid = &arrangement(&model);
    verify::arrangement(flow, topology, valid).expect("the arrangement conforms");

    for (name, mutate) in mutations() {
        let mut broken = valid.clone();
        mutate(&mut broken);
        assert!(
            verify::arrangement(flow, topology, &broken).is_err(),
            "the verifier accepted {name}"
        );
    }
}

/// The same topology keeps the same arrangement however often it is built.
#[test]
fn the_construction_is_deterministic() {
    let source = looping(&["repeat", "repeat", "break"]);
    let first = arrangement(&model(&source));
    let second = arrangement(&model(&source));
    assert_eq!(first.rank, second.rank);
    assert_eq!(first.ranks, second.ranks);
    assert_eq!(first.column, second.column);
    assert_eq!(first.exit_offset, second.exit_offset);
    assert_eq!(first.footprints, second.footprints);
    assert_eq!(first.routes, second.routes);
    assert_eq!(first.gap_lanes, second.gap_lanes);
    assert_eq!(first.contours, second.contours);
    // Nothing else is recorded, so nothing else can differ.
    let Arrangement {
        rank: _,
        ranks: _,
        column: _,
        exit_offset: _,
        footprints: _,
        routes: _,
        gap_lanes: _,
        contours: _,
    } = first;
}

#[test]
fn failure_to_construct_does_not_reject_the_model() {
    let model = model(&looping(&["repeat", "break", "repeat"]));
    let error = super::construct(&model.flow, &model.merges, &model.topology)
        .err()
        .expect("the enclosed break cannot be routed");
    assert!(error.to_string().contains("could not construct a diagram"));
}

#[test]
fn nested_break_routes_merge_without_crossing_side_departures() {
    let source = include_str!("../../../kaalang/tests/loop/behavior/nested_break_routes.rs");
    let model = crate::build(&crate::tests::fixture(source, "nested_break_routes")).unwrap();
    let built = arrangement(&model);
    verify::arrangement(&model.flow, &model.topology, &built).unwrap();
    // The first exit must postpone its turn past the inner questions' exits.
    let outer_exit = model
        .topology
        .connections
        .iter()
        .position(|wire| {
            wire.source
                == Source::Exit(ExitId {
                    node: NodeId::Block(1),
                    branch: Some(1),
                })
        })
        .unwrap();
    assert!(built.routes[outer_exit].runs[0].gap > built.rank[&Vertex::Node(NodeId::Block(2))]);
}
