use std::collections::BTreeSet;

pub(super) use kaalang_testing::shapes::looping;
use kaalang_testing::shapes::{declared_domain, flat_bodies, loop_shapes, nested, question_shapes};

use super::{Arrangement, Contour, Side, place, verify};
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

/// The deciding sweep's checked arrangement for one flow, or the message of
/// the topology it refuses. An internal refusal, or an arrangement its own
/// check rejects, is a defect either way, and panics with `label`.
fn swept(
    label: &str,
    flow: &Flow,
    merges: &[WireMerge],
    topology: &Topology,
) -> Result<Arrangement, String> {
    match super::sweep::search(flow, merges, topology) {
        Ok(built) => {
            verify::arrangement(flow, topology, &built).unwrap_or_else(|reason| {
                panic!("{label}\nthe sweep drew an invalid arrangement: {reason}")
            });
            Ok(built)
        }
        Err(super::sweep::Refusal::Impossible(blocked)) => Err(blocked.message),
        Err(super::sweep::Refusal::Internal(reason)) => panic!("{label}\n{reason}"),
    }
}

/// The parts the arrangement reads, so a test can rebuild a candidate.
pub(super) struct Parts {
    pub(super) flow: Flow,
    pub(super) merges: Vec<WireMerge>,
    pub(super) topology: Topology,
}

/// The analyzed parts of a flow, stopping short of construction.
///
/// `build` constructs, so reading the parts from it would hide every flow the
/// construction refuses — which is exactly the set a decision test has to see.
/// `None` means a rule other than realizability rejected the flow.
pub(super) fn parts_of(source: &str) -> Option<Parts> {
    parts_from(&syn::parse_str(source).ok()?)
}

/// The same, for a function a caller has already parsed.
pub(super) fn parts_from(function: &syn::ItemFn) -> Option<Parts> {
    let mut flow = crate::parse::flow(function).ok()?;
    crate::scope::resolve(&mut flow).ok()?;
    crate::resolve::flow(&flow).ok()?;
    let (executions, _, merges) = crate::analyze::flow(&flow).ok()?;
    let execution_plan = crate::plan::flow(&flow, &executions, &merges);
    let topology = crate::topology::project(
        &crate::topology::Analyzed {
            flow: &flow,
            executions: &executions,
            merges: &merges,
            execution_plan: &execution_plan,
        },
        false,
    );
    Some(Parts {
        flow,
        merges,
        topology,
    })
}

/// Pins each three-case shape's public result. Counts alone could hide one
/// acceptance and one refusal swapping places.
#[test]
fn every_three_case_cycle_body_settles_the_same_way() {
    // Four shapes violate semantic rules; two enclose an exit between
    // converging routes when the cases occupy their mandatory common row.
    let refused = [
        ["repeat", "break", "repeat"],
        ["repeat", "finish", "repeat"],
        ["break", "repeat", "break"],
        ["break", "repeat", "finish"],
        ["finish", "repeat", "break"],
        ["finish", "repeat", "finish"],
    ];
    let mut settled = Vec::new();
    for first in ["repeat", "break", "finish"] {
        for second in ["repeat", "break", "finish"] {
            for third in ["repeat", "break", "finish"] {
                let routes = [first, second, third];
                let function: syn::ItemFn =
                    syn::parse_str(&looping(&routes)).expect("the probe parses");
                let drawn = crate::build(&function).is_ok();
                assert_eq!(
                    drawn,
                    !refused.contains(&routes),
                    "{routes:?}: {} unexpectedly",
                    if drawn { "drawn" } else { "refused" }
                );
                settled.push(drawn);
            }
        }
    }
    assert_eq!(settled.len(), 27, "every shape should settle");
}

/// Semantic obstructions retain their diagnostic priority independently of
/// construction, including ordinary convergence rules for cycle exit routes.
#[test]
fn semantic_restrictions_keep_their_specific_diagnostics() {
    for (routes, expected) in [
        // Both completing routes provide the wire captured by the single break.
        (
            ["finish", "repeat", "break"],
            "branches in a kaalang choice convergence group must be adjacent",
        ),
        (
            ["break", "repeat", "break"],
            "branches in a kaalang choice convergence group must be adjacent",
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
fn a_cycle_takes_the_clear_contour_whatever_the_preference() {
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

/// A terminal route between repeats cannot escape while the cases share a row.
#[test]
fn a_terminal_route_between_two_repeats_is_rejected() {
    let source = looping(&["repeat", "finish", "repeat"]);
    let function = syn::parse_str(&source).unwrap();
    let error = crate::build(&function)
        .err()
        .expect("the flow has no diagram");
    assert!(error.to_string().contains("no conforming arrangement"));
}

/// A cycle is a projection bug, not an undrawable flow: ranking refuses it and
/// the search reports it as an internal error rather than blaming the author.
#[test]
fn a_cycle_is_refused_rather_than_ranked() {
    let chain = place::rows(&linked(&[(0, 1)]), &BTreeSet::new()).expect("a chain has an order");
    assert!(chain[&Vertex::Node(NodeId::Block(0))] < chain[&Vertex::Node(NodeId::Block(1))]);

    assert_eq!(
        place::rows(&linked(&[(0, 1), (1, 0)]), &BTreeSet::new()),
        Err("the connections form a cycle, so no node can be lowest".to_owned())
    );
}

/// One named break of a valid arrangement.
type Mutation = (&'static str, Box<dyn Fn(&mut Arrangement)>);

/// Mutations covering arrangement structure and geometry.
fn mutations() -> Vec<Mutation> {
    let mut mutations = coverage_mutations();
    mutations.extend(geometry_mutations());
    mutations
}

/// Breaks structural coverage before geometry is checked.
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
            "a contour climbing between the columns of its body",
            Box::new(|arrangement: &mut Arrangement| {
                // The far edge of the cases, so the climb lands inside the
                // body's column range however much room the arrangement left
                // beside it.
                let cases = arrangement
                    .column
                    .iter()
                    .filter(|(vertex, _)| matches!(vertex, Vertex::Node(NodeId::Case { .. })))
                    .map(|(_, column)| *column);
                let (inside, outside) = cases.fold((i32::MAX, i32::MIN), |(low, high), column| {
                    (low.min(column), high.max(column))
                });
                if let Some(contour) = arrangement.contours.first_mut() {
                    contour.column = match contour.side {
                        Side::Left => outside,
                        Side::Right => inside,
                    };
                }
            }),
        ),
        (
            "a contour one lane past the last a topology offers",
            Box::new(|arrangement: &mut Arrangement| {
                // Each cycle takes a lane for its back edge and the next one
                // out for the boundary around it, so a topology offers twice
                // its cycle count. This probe has one cycle, so lane 2 is
                // already one too many.
                if let Some(contour) = arrangement.contours.first_mut() {
                    contour.lane += 2;
                }
            }),
        ),
    ]
}

/// Each mutation must fail verification. This probe cannot isolate crossings:
/// its single-lane gaps produce diagonal routes first. Impossibility tests and
/// shared geometry tests cover crossing rejection separately.
#[test]
fn the_verifier_rejects_every_mutation() {
    // Both a cycle break and flow completion outside the repeating branches.
    for routes in [
        ["repeat", "repeat", "break"].as_slice(),
        ["repeat", "repeat", "finish", "finish"].as_slice(),
    ] {
        let model = model(&looping(routes));
        let flow = &model.flow;
        let topology = &model.topology;
        let valid = &model.arrangement;
        verify::arrangement(flow, topology, valid).expect("the arrangement conforms");

        for (name, mutate) in mutations() {
            let mut broken = valid.clone();
            mutate(&mut broken);
            assert!(
                verify::arrangement(flow, topology, &broken).is_err(),
                "{routes:?}: the verifier accepted {name}"
            );
        }
    }
}

/// Node dimensions and label text are a presentation's business. The same
/// topology under long, wide, non-Latin descriptions keeps the arrangement it
/// had under short ones.
#[test]
fn descriptions_do_not_change_the_arrangement() {
    let plain = looping(&["repeat", "repeat", "break"]);
    let first = model(&plain).arrangement;
    // Everything a presentation measures and the topology does not: the
    // description text and its script, the authored return type, and the
    // parameter list.
    for wordy in [
        plain.replace(
            "Case ",
            "Разверните шаг и продолжайте, пока счётчик не дойдёт до конца — case ",
        ),
        plain.replace("-> u8", "-> ::std::collections::BTreeMap<String, Vec<u8>>"),
        plain.replace(
            "fn probe(mode: u8)",
            "fn probe(mode: u8, _unused: &'static [(u8, u8)], _also: Option<Box<u8>>)",
        ),
    ] {
        assert_ne!(plain, wordy, "each variation should change the source");
        assert_eq!(first, model(&wordy).arrangement);
    }
}

/// The same topology keeps the same arrangement however often it is built.
#[test]
fn the_construction_is_deterministic() {
    let source = looping(&["repeat", "repeat", "break"]);
    let first = arrangement(&model(&source));
    assert_eq!(first, arrangement(&model(&source)));
}

#[test]
fn nested_break_routes_merge_without_crossing_side_departures() {
    let source = include_str!("../../../../kaalang/tests/loop/behavior/nested_break_routes.rs");
    let model = crate::build(&crate::tests::fixture(source, "nested_break_routes")).unwrap();
    let built = arrangement(&model);
    verify::arrangement(&model.flow, &model.topology, &built).unwrap();
    // The first exit reaches the shared wire merge in its own branch column.
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
    assert!(built.routes[outer_exit].runs.is_empty());
}

/// Exercises the sweep directly on accepted fixtures, bypassing the preferred
/// search. Every result must pass independent verification.
#[test]
fn the_sweep_alone_draws_every_fixture_the_model_accepts() {
    let mut checked = 0;
    let corpus = kaalang_testing::corpus::corpus();
    kaalang_testing::corpus::assert_whole_tree(&corpus);
    for (name, function) in corpus {
        let Ok(model) = crate::build(&function) else {
            continue;
        };
        checked += 1;
        swept(&name, &model.flow, &model.merges, &model.topology).unwrap_or_else(|blocked| {
            panic!("{name}: the sweep refuses a drawable flow: {blocked}")
        });
    }
    assert!(checked > 100, "the sweep should reach most of the corpus");
}

#[test]
fn no_generated_cycle_shape_reaches_an_internal_error() {
    let shapes = loop_shapes();
    let (mut drawn, mut crossed) = (0, 0);
    for source in &shapes {
        let function: syn::ItemFn = syn::parse_str(source).expect("the probe parses");
        match crate::build(&function) {
            Ok(mut model) => {
                drawn += 1;
                verify::arrangement(&model.flow, &model.topology, &model.arrangement)
                    .unwrap_or_else(|reason| panic!("{source}\n{reason}"));
                // Compaction trades the row and serial rules for the boundary
                // rule, so a compacted witness answers to the compacting
                // verifier and never to the one that built it. The searches
                // stay unaware of the rectangle on purpose, so an arrangement
                // may reach compaction already crossing one; what compaction
                // owes is not to make that worse.
                let shape = verify::Shape::of(&model.flow, &model.topology);
                let given =
                    verify::compacted(&model.flow, &model.topology, &model.arrangement, &shape);
                model.compact_arrangement();
                if let Err(reason) =
                    verify::compacted(&model.flow, &model.topology, &model.arrangement, &shape)
                {
                    assert!(given.is_err(), "compaction broke {source}\n{reason}");
                    crossed += 1;
                }
            }
            Err(error) => assert!(
                !error.to_string().contains("internal kaalang"),
                "{source}\n{error}"
            ),
        }
    }
    assert!(drawn > 100, "too few of the generated shapes were drawn");
    // One shape reaches compaction already crossing a boundary: an outer cycle
    // of `repeat/inner/repeat` over an inner cycle of `repeat/repeat`, where
    // the last case's route passes through the inner body's columns. Pinned so
    // the gap the searches leave cannot widen unnoticed.
    assert_eq!(
        crossed, 1,
        "the boundary rule is crossed by more shapes now"
    );
}

/// Exercises the sweep directly on accepted generated cycles, including nested
/// back edges with disjoint row spans. Every result must pass verification.
#[test]
fn the_sweep_alone_draws_every_generated_shape_the_model_accepts() {
    let mut checked = 0;
    for source in &loop_shapes() {
        let function: syn::ItemFn = syn::parse_str(source).expect("the probe parses");
        let Ok(model) = crate::build(&function) else {
            continue;
        };
        checked += 1;
        swept(source, &model.flow, &model.merges, &model.topology).unwrap_or_else(|blocked| {
            panic!("{source}\nthe sweep refuses a drawable flow: {blocked}")
        });
    }
    assert!(checked > 100, "too few of the generated shapes were drawn");
}

/// A back edge climbs outside every back edge nested in its body, whether or not
/// the two share a row.
///
/// Here the outer back edge spans the rows above the inner cycle and the inner
/// back edge the rows below it, so the two can never cross. Only comparing where
/// they climb catches an outer back edge left inside the body it leaves.
#[test]
fn a_back_edge_clears_a_nested_back_edge_it_cannot_cross() {
    let source = "fn probe(mode: u8) {
    #[cycle(\"Repeat the outer cycle.\")]
    |mode| {
        #[question(\"Repeat?\")]
        let (again, enter) = |mode| mode == 0;
        #[action(\"Repeat.\")]
        |again| ();
        #[action(\"First.\")]
        let first = |enter| ();
        #[action(\"Second.\")]
        let second = |first| ();
        #[action(\"Third.\")]
        let third = |second| ();
        #[cycle(\"Diverge in the inner cycle.\")]
        |third| {};
    };
}

";
    let model = model(source);
    let topology = &model.topology;
    verify::arrangement(&model.flow, topology, &model.arrangement).expect("the probe conforms");
    let outer = &model.arrangement.contours[0];
    let inner = &model.arrangement.contours[1];
    assert_eq!(
        outer.side, inner.side,
        "both back edges take the same flank"
    );
    let entry =
        |loop_: &crate::topology::Loop| model.arrangement.rank[&Vertex::Junction(loop_.entry)];
    let tail =
        |loop_: &crate::topology::Loop| model.arrangement.rank[&Vertex::Junction(loop_.tail)];
    assert!(
        entry(&topology.loops[0]) > tail(&topology.loops[1])
            || entry(&topology.loops[1]) > tail(&topology.loops[0]),
        "the two back edges should span rows that cannot cross"
    );

    // Move the inner back edge one step further out than the outer one. Nothing
    // crosses, and the outer back edge is now inside the body it leaves.
    let mut broken = model.arrangement.clone();
    broken.contours[1] = Contour {
        side: outer.side,
        column: outer.column,
        lane: outer.lane + 1,
    };
    let error = verify::arrangement(&model.flow, topology, &broken)
        .expect_err("the outer back edge no longer clears the nested one");
    assert!(error.contains("climbs inside its body"), "{error}");
}

mod reference;

/// Distributor routes may share rails, but all their case endpoints must
/// arrive on one row. Neither search may hide an enclosed exit below the tail.
#[test]
fn exits_between_repeating_cases_are_refused_by_both_procedures() {
    for routes in [
        ["repeat", "break", "repeat"].as_slice(),
        ["repeat", "finish", "repeat"].as_slice(),
        ["repeat", "finish", "finish", "repeat"].as_slice(),
    ] {
        let source = looping(routes);
        let parts = parts_of(&source).expect("the flow is semantically valid");
        assert!(
            matches!(
                super::sweep::search(&parts.flow, &parts.merges, &parts.topology),
                Err(super::sweep::Refusal::Impossible(_))
            ),
            "{routes:?}"
        );
        assert!(
            !reference::admissible(&parts.flow, &parts.topology),
            "{routes:?}"
        );
        let function = syn::parse_str(&source).unwrap();
        assert!(crate::build(&function).is_err(), "{routes:?}");
    }
}

#[test]
fn authors_can_reorder_enclosed_cases_to_restore_a_drawing() {
    for routes in [
        ["repeat", "repeat", "break"].as_slice(),
        ["repeat", "repeat", "finish"].as_slice(),
        ["repeat", "repeat", "finish", "finish"].as_slice(),
        ["break", "break", "finish"].as_slice(),
    ] {
        let model = model(&looping(routes));
        let cases = (0..routes.len())
            .map(|branch| Vertex::Node(NodeId::Case { choice: 1, branch }))
            .collect::<Vec<_>>();
        for pair in cases.windows(2) {
            assert_eq!(
                model.arrangement.rank[&pair[0]],
                model.arrangement.rank[&pair[1]]
            );
            assert!(model.arrangement.column[&pair[0]] < model.arrangement.column[&pair[1]]);
        }
    }
}

/// Decision-test shapes: flat bodies of two to four routes, nested loops, and
/// ordered question ports, including both drawable and impossible cases.
pub(super) fn decision_cases() -> Vec<String> {
    let names = ["repeat", "break", "finish"];
    let mut cases = flat_bodies(2..=4);
    for inner in [
        ["repeat", "break"].as_slice(),
        ["break", "repeat"].as_slice(),
        ["propagate", "repeat"].as_slice(),
        ["repeat", "propagate"].as_slice(),
        ["finish", "repeat"].as_slice(),
        ["repeat", "break", "repeat"].as_slice(),
    ] {
        for other in names {
            cases.push(nested(&["inner", other], inner));
            cases.push(nested(&[other, "inner"], inner));
        }
    }
    cases.extend(question_shapes());
    cases
}

/// The flat two- and three-route bodies of [`decision_cases`], with the
/// ordered question ports. Named rather than counted off the front of
/// `decision_cases`, which would silently follow it when its domain widens.
pub(super) fn small_decision_cases() -> Vec<String> {
    flat_bodies(2..=3)
        .into_iter()
        .chain(question_shapes())
        .collect()
}

/// Compares acceptances and refusals with an independent procedure for every
/// flat body of two or three routes. A mismatch is a false refusal or unsound
/// acceptance. The wider domain runs in the ignored test below.
#[test]
fn the_construction_agrees_with_an_independent_procedure() {
    let counted = agree(&flat_bodies(2..=3));
    assert!(
        counted.drawn == 30 && counted.refused == 2 && counted.rejected_earlier == 4,
        "the flat domain should preserve every known outcome: {counted:?}"
    );
}

/// Exhaustive comparison, excluded from default runs due to its cost.
///
/// Run it with:
/// `cargo test -p kaalang-compiler --lib the_declared_domain_agrees -- --ignored --nocapture`
#[test]
#[ignore = "exhaustive; run it with --ignored"]
fn the_declared_domain_agrees_with_the_independent_procedure() {
    let counted = agree(&declared_domain());
    println!("{counted:?}");
    assert!(
        counted.drawn == 239 && counted.refused == 70 && counted.rejected_earlier == 240,
        "the declared distributor domain preserves every independently checked answer: {counted:?}"
    );
}

/// What one comparison saw.
#[derive(Debug, Default)]
struct Counted {
    drawn: usize,
    refused: usize,
    rejected_earlier: usize,
}

/// Compares sweep acceptances and refusals with the independent reference.
/// Bypasses preferred construction and skips flows rejected by semantic rules.
fn agree(cases: &[String]) -> Counted {
    let mut counted = Counted::default();
    for source in cases {
        let Some(parts) = parts_of(source) else {
            counted.rejected_earlier += 1;
            continue;
        };
        // An arrangement the deciding search returns and its own check
        // rejects is an internal error, and counting it as a refusal is what
        // would hide it: a topology that has no diagram either way makes the
        // reference agree and the comparison pass.
        let drawn = swept(source, &parts.flow, &parts.merges, &parts.topology).is_ok();
        let flexible =
            match super::sweep::search_shape(&parts.flow, &parts.merges, &parts.topology, true) {
                Ok(_) => true,
                Err(super::sweep::Refusal::Impossible(_)) => false,
                Err(super::sweep::Refusal::Internal(reason)) => {
                    panic!("{source}\nflexible back edges: {reason}")
                }
            };
        assert_eq!(
            flexible, drawn,
            "{source}\nstraight preference must not hide the general search"
        );
        if drawn {
            counted.drawn += 1;
        } else {
            counted.refused += 1;
        }
        assert_eq!(
            reference::admissible(&parts.flow, &parts.topology),
            drawn,
            "{source}"
        );
    }
    counted
}

#[test]
fn the_verifier_rejects_a_case_on_a_different_row() {
    let model = model(&looping(&["repeat", "repeat", "break"]));
    let mut broken = model.arrangement.clone();
    let case = Vertex::Node(NodeId::Case {
        choice: 1,
        branch: 2,
    });
    *broken.rank.get_mut(&case).unwrap() += 1;
    let error = verify::arrangement(&model.flow, &model.topology, &broken).unwrap_err();
    assert_eq!(error, "choice 2 draws its cases on different rows");
}

#[test]
fn body_columns_do_not_shrink_when_a_body_vertex_moves_below_the_tail() {
    let model = model(&looping(&["repeat", "finish"]));
    let loop_ = model.topology.loops[0];
    let mut placed = model.arrangement.clone();
    let vertex = Vertex::Node(NodeId::Case {
        choice: loop_.header + 1,
        branch: 1,
    });
    let column = placed.column.values().max().unwrap() + 1;
    placed.column.insert(vertex, column);
    placed
        .rank
        .insert(vertex, placed.rank[&Vertex::Junction(loop_.tail)] + 1);
    assert!(
        super::loop_block::body_columns(&model.flow, &model.topology, &placed, loop_.header)
            .contains(&column)
    );
}

#[test]
fn reaching_an_enclosing_tail_does_not_make_it_part_of_the_inner_body() {
    let model = model(
        "fn probe(flag: bool) {
            #[cycle(\"Repeat the outer cycle.\")]
            |flag| {
                #[cycle(\"Repeat or leave the inner cycle.\")]
                |flag| {
                    #[question(\"Repeat?\")]
                    let (again, leave) = |flag| flag;
                    |leave| break;
                    #[action(\"Repeat.\")]
                    |again| ();
                };
            };
        }",
    );
    let [outer, inner] = model.topology.loops[..] else {
        panic!("the flow has two repeating cycles");
    };
    let body = model.body_vertices(inner.header);
    assert!(body.contains(&Vertex::Junction(inner.tail)));
    assert!(!body.contains(&Vertex::Junction(outer.tail)));
}

#[test]
fn a_nested_result_and_its_following_break_belong_to_the_outer_body() {
    let file = syn::parse_file(include_str!(
        "../../../../kaalang/tests/loop/behavior/conditional_nested_loop.rs"
    ))
    .unwrap();
    let function = file
        .items
        .into_iter()
        .find_map(|item| match item {
            syn::Item::Fn(function) if function.sig.ident == "conditional_nested_loop" => {
                Some(function)
            }
            _ => None,
        })
        .unwrap();
    let model = crate::build(&function).expect("the flow is valid");
    let [outer, inner] = model.topology.loop_boundaries[..] else {
        panic!("the flow has two cycle boundaries");
    };
    let result = inner
        .result
        .map(Vertex::from)
        .expect("the inner cycle completes");
    let body = model.body_vertices(outer.header);
    assert!(body.contains(&result));
    assert!(
        model
            .topology
            .outgoing(result)
            .all(|edge| body.contains(&edge.destination))
    );
}

/// Regression for constraining an earlier sibling against a later convergence.
/// Branch 2 diverges between routes meeting at the tail; requiring it to stay
/// left of that tail falsely rejects the drawable `diverging_middle_branch` fixture.
const DIVERGING_MIDDLE_BRANCH: &str = "fn diverging_middle_branch(mode: u8, stay: bool) -> u8 {
    #[cycle(\"Choose a repeating, diverging, or leaving route.\")]
    let result = |mode, stay| {
        #[choice(\"Which route?\")]
        #[case(\"Advance and repeat.\")]
        #[case(\"Spin forever.\")]
        #[case(\"Advance, then repeat or leave.\")]
        #[case(\"Leave at once.\")]
        let (advance, spin, decide, leave_now) = |mode| match mode {
            0 => (),
            1 => (),
            2 => (),
            _ => (),
        };
        #[action(\"Advance.\")]
        |advance| {};
        #[cycle(\"Spin forever.\")]
        |spin| {
            #[action(\"Spin.\")]
            || {};
        };
        #[question(\"Stay in the loop?\")]
        let (again, leave) = |decide, stay| stay;
        #[action(\"Advance after the decision.\")]
        |again| {};
        #[action(\"Leave after the decision.\")]
        let selected = |leave, mode| mode;
        #[action(\"Leave immediately.\")]
        let selected = |leave_now, mode| mode;
        |selected| break selected;
    };
    |result| return result;
}
";

#[test]
fn a_diverging_branch_between_partial_merges_is_drawn() {
    let parts = parts_of(DIVERGING_MIDDLE_BRANCH).expect("the flow passes the earlier rules");
    swept(
        DIVERGING_MIDDLE_BRANCH,
        &parts.flow,
        &parts.merges,
        &parts.topology,
    )
    .unwrap_or_else(|blocked| panic!("the sweep refuses a drawable flow: {blocked}"));
    model(DIVERGING_MIDDLE_BRANCH);
    assert!(
        reference::admissible(&parts.flow, &parts.topology),
        "the independent procedure should find a construction too"
    );
}

/// Pins eight nested-loop regressions from the generated domain so failures
/// identify a named shape. Each must reach a decision without an internal error.
#[test]
fn every_audit_shape_reaches_a_decision() {
    let cases: [(usize, &[&str], &[&str]); 8] = [
        (1, &["inner", "repeat", "repeat"], &["propagate", "repeat"]),
        (2, &["break", "inner", "repeat"], &["propagate", "repeat"]),
        (3, &["finish", "inner", "repeat"], &["propagate", "repeat"]),
        (4, &["break", "inner", "repeat"], &["finish", "repeat"]),
        (5, &["finish", "inner", "repeat"], &["finish", "repeat"]),
        (6, &["break", "inner", "repeat"], &["propagate", "break"]),
        (7, &["repeat", "finish", "inner"], &["finish", "break"]),
        (8, &["finish", "inner", "repeat"], &["finish", "break"]),
    ];
    for (case, outer, inner) in cases {
        let source = nested(outer, inner);
        let function: syn::ItemFn = syn::parse_str(&source).expect("the probe parses");
        match crate::build(&function) {
            Ok(model) => verify::arrangement(&model.flow, &model.topology, &model.arrangement)
                .unwrap_or_else(|reason| panic!("audit case {case}: {reason}")),
            Err(error) => assert!(
                !error.to_string().contains("internal kaalang"),
                "audit case {case}: {error}"
            ),
        }
        // The cycle contract adds explicit result propagation, so some old
        // rows now settle in semantic analysis. This pins the public decision:
        // every generated source is accepted or rejected without an internal
        // construction error.
    }
}

/// A back edge one column beyond its body still verifies. The renderer uses
/// this witness to check that the recorded column survives realization.
#[test]
fn a_back_edge_may_stand_beyond_the_body_it_clears() {
    let model = model(&looping(&["repeat", "repeat", "break"]));
    let columns = || model.arrangement.column.values().copied();
    let contour = model.arrangement.contours[0];
    let beyond = match contour.side {
        Side::Left => columns().min().expect("the probe has columns") - 2,
        Side::Right => columns().max().expect("the probe has columns") + 2,
    };

    let mut far = model.arrangement.clone();
    far.contours[0] = Contour {
        column: beyond,
        ..contour
    };
    verify::arrangement(&model.flow, &model.topology, &far)
        .expect("a back edge standing further out than its body still conforms");
}

/// The sole-arrival witness used by the SVG contour regression must also
/// pass the model's check before the renderer is held to its recorded column.
#[test]
fn a_back_edge_with_one_arrival_may_stand_beyond_its_body() {
    let function = crate::tests::fixture(
        include_str!("../../../../kaalang/tests/loop/behavior/reversed_empty_loop.rs"),
        "reversed_empty_loop",
    );
    let mut model = crate::build(&function).expect("the fixture is valid");
    assert_eq!(model.arrangement.contours[0].side, Side::Right);
    model.arrangement.contours[0].column = model.arrangement.column.values().max().unwrap() + 2;
    verify::arrangement(&model.flow, &model.topology, &model.arrangement)
        .expect("the farther contour is a valid witness for SVG");
}

/// Four cycles nested one inside the next, whose back edges climb the same side in
/// four lanes. The outermost takes lane 3, which no fixture reaches.
pub(super) const FOUR_LANES: &str = "fn deep(mut step: usize) -> usize {
    #[cycle(\"Repeat the first cycle.\")]
    |step| {
        #[question(\"Leave the first?\")]
        let (stay_0, leave_0) = |&step| *step > 0;
        |leave_0| break;
        #[cycle(\"Repeat the second cycle.\")]
        |stay_0, step| {
            #[question(\"Leave the second?\")]
            let (stay_1, leave_1) = |&step| *step > 1;
            |leave_1| break;
            #[cycle(\"Repeat the third cycle.\")]
            |stay_1, step| {
                #[question(\"Leave the third?\")]
                let (stay_2, leave_2) = |&step| *step > 2;
                |leave_2| break;
                #[cycle(\"Repeat the fourth cycle.\")]
                |stay_2, mut step| {
                    #[question(\"Leave the fourth?\")]
                    let (stay_3, leave_3) = |&step| *step > 3;
                    |leave_3| break;
                    #[action(\"Advance at the deepest level.\")]
                    |stay_3, &mut step| *step += 1;
                };
            };
        };
    };
    |step| return step;
}
";

/// Packs four nested back edges into lanes 0–3, beyond generated coverage.
/// The lane budget also reserves cycle boundaries; `kaalang-svg` draws this witness.
#[test]
fn four_nested_back_edges_climb_four_lanes_on_one_side() {
    let model = model(FOUR_LANES);
    assert_eq!(model.topology.loops.len(), 4, "four nested loops");

    let side = model.arrangement.contours[0].side;
    let mut deep = model.arrangement.clone();
    for (index, contour) in deep.contours.iter_mut().enumerate() {
        contour.side = side;
        contour.lane = deep_lane(index);
    }
    verify::arrangement(&model.flow, &model.topology, &deep)
        .expect("four back edges in four lanes on one side conform");
    assert_eq!(
        deep.contours[0].lane, 3,
        "the outermost takes the last lane"
    );
}

/// Which lane the back edge of the `FOUR_LANES` cycle at `index` climbs: the
/// outermost is furthest out, so the lanes run down with the nesting.
pub(super) const fn deep_lane(index: usize) -> usize {
    3 - index
}

#[test]
fn ordered_question_ports_cover_genuine_refusals_in_both_procedures() {
    let counted = agree(&question_shapes());
    assert_eq!(
        (counted.drawn, counted.refused, counted.rejected_earlier),
        (21, 2, 4)
    );
}

#[test]
fn value_producing_exit_routes_share_the_merge_before_break() {
    let counted = agree(&[looping(&["break", "finish", "break", "repeat"])]);
    assert_eq!(counted.drawn, 1);
}

#[test]
fn junction_arrivals_record_their_rank_through_compaction() {
    let merge = crate::tests::fixture(
        include_str!("../../../../kaalang/tests/wire/behavior/two_merges_reach_one_consumer.rs"),
        "two_merges_reach_one_consumer",
    );
    for (function, minimum) in [
        (merge, 2),
        (
            syn::parse_str(&looping(&["repeat", "repeat", "break"])).unwrap(),
            1,
        ),
    ] {
        let model = crate::build(&function).unwrap();
        let swept = super::sweep::search(&model.flow, &model.merges, &model.topology)
            .ok()
            .expect("the sweep can draw the junction arrivals");
        for mut built in [model.arrangement.clone(), swept] {
            for compact in [false, true] {
                if compact {
                    super::compact::arrangement(&model.flow, &model.topology, &mut built);
                }
                verify::arrangement(&model.flow, &model.topology, &built).unwrap();
                let mut arrivals = 0;
                for (index, wire) in model.topology.connections.iter().enumerate() {
                    if !matches!(wire.destination, Vertex::Junction(_)) {
                        continue;
                    }
                    if let Some(run) = built.routes[index].runs.last() {
                        assert_eq!(
                            run.line,
                            super::RunLine::Rank(built.rank[&wire.destination])
                        );
                        // Reject both an absent row and a real row below the junction;
                        // verification must not snap the recorded run back to its endpoint.
                        for rank in [built.ranks, built.rank[&wire.destination] + 1] {
                            let mut broken = built.clone();
                            broken.routes[index].runs.last_mut().unwrap().line =
                                super::RunLine::Rank(rank);
                            assert!(
                                verify::arrangement(&model.flow, &model.topology, &broken).is_err()
                            );
                        }
                        arrivals += 1;
                    }
                }
                assert!(
                    arrivals >= minimum,
                    "the fixture exercises sideways junction arrivals"
                );
            }
        }
    }
}

#[test]
fn a_back_edge_can_bend_outside_its_body() {
    let model = model(&looping(&["repeat", "repeat", "break"]));
    let mut bent = model.arrangement.clone();
    let contour = bent.contours[0];
    let delta = match contour.side {
        Side::Left => -1,
        Side::Right => 1,
    };
    let loop_ = model.topology.loops[0];
    let entry = bent.rank[&Vertex::Junction(loop_.entry)];
    let tail = bent.rank[&Vertex::Junction(loop_.tail)];
    let gap = usize::midpoint(entry, tail);
    let lane = bent.gap_lanes[gap];
    bent.gap_lanes[gap] += 1;
    bent.back_routes.insert(
        0,
        super::Route {
            departure: contour.column,
            arrival: contour.column + delta,
            runs: vec![super::Run {
                line: super::RunLine::Lane { gap, lane },
                enter: contour.column,
                exit: contour.column + delta,
            }],
        },
    );
    verify::arrangement(&model.flow, &model.topology, &bent).unwrap();
    let mut wrong_entry = bent.clone();
    wrong_entry.contours[0].column += 1;
    assert!(
        verify::arrangement(&model.flow, &model.topology, &wrong_entry)
            .unwrap_err()
            .contains("recorded entry column")
    );
    bent.back_routes.get_mut(&0).unwrap().runs[0].line = super::RunLine::Lane {
        gap,
        lane: lane + 1,
    };
    assert!(
        verify::arrangement(&model.flow, &model.topology, &bent)
            .unwrap_err()
            .contains("takes lane")
    );
}

#[test]
fn serial_nodes_do_not_need_artificial_side_steps() {
    let source = r#"fn straight(input: u8) -> u8 {
        #[action("Read the value.")]
        let value = |input| input;
        |value| return value;
    }"#;
    let parts = parts_of(source).unwrap();
    let Ok(built) = super::sweep::search(&parts.flow, &parts.merges, &parts.topology) else {
        panic!("a serial flow has a diagram");
    };
    assert!(built.routes.iter().all(|route| route.runs.is_empty()));
    assert!(built.gap_lanes.iter().all(|&count| count == 0));
}
