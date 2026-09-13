use std::collections::BTreeSet;

#[path = "../../../../kaalang/tests/support/diagram_shapes.rs"]
mod diagram_shapes;
pub(super) use diagram_shapes::looping;
use diagram_shapes::{declared_domain, loop_shapes, nested};

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
    let topology = crate::topology::project(&crate::topology::Analyzed {
        flow: &flow,
        executions: &executions,
        merges: &merges,
        execution_plan: &execution_plan,
    });
    Some(Parts {
        flow,
        merges,
        topology,
    })
}

/// Every shape of a three-case loop body, taken through the public entry
/// point, settles the way it is recorded here.
///
/// The outcome of each shape is written out rather than counted: `build`
/// checks the arrangement it returns, so re-checking it here would assert
/// nothing, and counting the accepted ones lets a shape flip from drawn to
/// refused without a test noticing. The sweep decides realizability, so the
/// agreement test for that decision lives beside it in `sweep`.
#[test]
fn every_three_case_loop_body_settles_the_same_way() {
    // Four shapes violate semantic rules; three enclose an exit between
    // converging routes when the cases occupy their mandatory common row.
    let refused = [
        ["break", "end", "break"],
        ["repeat", "break", "repeat"],
        ["repeat", "end", "repeat"],
        ["break", "repeat", "break"],
        ["break", "repeat", "end"],
        ["end", "repeat", "break"],
        ["end", "repeat", "end"],
    ];
    let mut settled = Vec::new();
    for first in ["repeat", "break", "end"] {
        for second in ["repeat", "break", "end"] {
            for third in ["repeat", "break", "end"] {
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

/// A terminal route between repeats cannot escape while the cases share a row.
#[test]
fn a_terminal_route_between_two_repeats_is_rejected() {
    let source = looping(&["repeat", "end", "repeat"]);
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
                // One return per loop can climb beside one column, so the last
                // lane a topology offers is its loop count minus one. This
                // probe has one loop, so lane 1 is already one too many.
                if let Some(contour) = arrangement.contours.first_mut() {
                    contour.lane += 1;
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
    // Both a loop exit and terminal routes outside the repeating branches.
    for routes in [
        ["repeat", "repeat", "break"].as_slice(),
        ["repeat", "repeat", "end", "end"].as_slice(),
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
        let second = model(&wordy).arrangement;
        assert_eq!(first.rank, second.rank);
        assert_eq!(first.ranks, second.ranks);
        assert_eq!(first.column, second.column);
        assert_eq!(first.exit_offset, second.exit_offset);
        assert_eq!(first.routes, second.routes);
        assert_eq!(first.gap_lanes, second.gap_lanes);
        assert_eq!(first.contours, second.contours);
        assert_eq!(first.return_routes, second.return_routes);
    }
    // Nothing else is recorded, so nothing else can differ.
    let Arrangement {
        rank: _,
        ranks: _,
        column: _,
        exit_offset: _,
        routes: _,
        gap_lanes: _,
        contours: _,
        return_routes: _,
    } = first;
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
    assert_eq!(first.routes, second.routes);
    assert_eq!(first.gap_lanes, second.gap_lanes);
    assert_eq!(first.contours, second.contours);
    assert_eq!(first.return_routes, second.return_routes);
    // Nothing else is recorded, so nothing else can differ.
    let Arrangement {
        rank: _,
        ranks: _,
        column: _,
        exit_offset: _,
        routes: _,
        gap_lanes: _,
        contours: _,
        return_routes: _,
    } = first;
}

#[test]
fn an_impossible_topology_rejects_the_model() {
    let function = crate::tests::fixture(
        include_str!(
            "../../../../kaalang/tests/loop/compile_fail/end_enclosed_by_nested_returns.rs"
        ),
        "end_enclosed_by_nested_returns",
    );
    let error = crate::build(&function)
        .err()
        .expect("the ordered question routes have no diagram");
    assert!(
        error
            .to_string()
            .starts_with("could not construct a diagram under RFC 0002")
    );
}

#[test]
fn nested_break_routes_merge_without_crossing_side_departures() {
    let source = include_str!("../../../../kaalang/tests/loop/behavior/nested_break_routes.rs");
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

/// The deciding sweep alone draws every fixture the model accepts, and every
/// arrangement it returns passes the independent check.
///
/// `construct` runs the preferred search first, so nothing else exercises the
/// sweep over real flows. Both halves matter: a refusal here would be a false
/// negative in the decision, and an arrangement the check rejects would be an
/// internal error waiting for the first flow the preferred search cannot draw.
#[test]
fn the_sweep_alone_draws_every_fixture_the_model_accepts() {
    let mut checked = 0;
    for (name, function) in crate::performance::corpus() {
        let Ok(model) = crate::build(&function) else {
            continue;
        };
        checked += 1;
        match super::sweep::search(&model.flow, &model.merges, &model.topology) {
            Err(super::sweep::Refusal::Impossible(blocked)) => panic!(
                "{name}: the sweep refuses a drawable flow: {}",
                blocked.message
            ),
            Err(super::sweep::Refusal::Internal(reason)) => panic!("{name}: {reason}"),
            Ok(built) => {
                verify::arrangement(&model.flow, &model.topology, &built).unwrap_or_else(
                    |reason| panic!("{name}: the sweep drew an invalid arrangement: {reason}"),
                );
            }
        }
    }
    assert!(checked > 100, "the corpus should be the whole tree");
}

#[test]
fn no_generated_loop_shape_reaches_an_internal_error() {
    let shapes = loop_shapes();
    let mut drawn = 0;
    for source in &shapes {
        let function: syn::ItemFn = syn::parse_str(source).expect("the probe parses");
        match crate::build(&function) {
            Ok(mut model) => {
                drawn += 1;
                verify::arrangement(&model.flow, &model.topology, &model.arrangement)
                    .unwrap_or_else(|reason| panic!("{source}\n{reason}"));
                model.compact_arrangement();
                verify::arrangement(&model.flow, &model.topology, &model.arrangement)
                    .unwrap_or_else(|reason| panic!("compacted {source}\n{reason}"));
            }
            Err(error) => assert!(
                !error.to_string().contains("internal kaalang"),
                "{source}\n{error}"
            ),
        }
    }
    assert!(drawn > 100, "too few of the generated shapes were drawn");
}

/// The deciding sweep alone draws every generated loop shape the model
/// accepts, and every arrangement it returns passes the independent check.
///
/// The test above answers through `build`, where the preferred search draws
/// nearly every shape and the sweep never runs. Nested shapes are what the
/// fixture corpus lacks: a loop whose return has to clear a return nested in
/// its body, over rows the two never share.
#[test]
fn the_sweep_alone_draws_every_generated_shape_the_model_accepts() {
    let mut checked = 0;
    for source in &loop_shapes() {
        let function: syn::ItemFn = syn::parse_str(source).expect("the probe parses");
        let Ok(model) = crate::build(&function) else {
            continue;
        };
        checked += 1;
        match super::sweep::search(&model.flow, &model.merges, &model.topology) {
            Err(super::sweep::Refusal::Impossible(blocked)) => panic!(
                "{source}\nthe sweep refuses a drawable flow: {}",
                blocked.message
            ),
            Err(super::sweep::Refusal::Internal(reason)) => panic!("{source}\n{reason}"),
            Ok(built) => {
                verify::arrangement(&model.flow, &model.topology, &built).unwrap_or_else(
                    |reason| panic!("{source}\nthe sweep drew an invalid arrangement: {reason}"),
                );
            }
        }
    }
    assert!(checked > 100, "too few of the generated shapes were drawn");
}

/// A return climbs outside every return nested in its body, whether or not
/// the two share a row.
///
/// Here the outer return spans the rows above the inner loop and the inner
/// return the rows below it, so the two can never cross. Only comparing where
/// they climb catches an outer return left inside the body it leaves.
#[test]
fn a_return_clears_a_nested_return_it_cannot_cross() {
    let source = "fn probe(mode: u8) {
    loop {
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
        |third| loop {};
    }
}

";
    let model = model(source);
    let topology = &model.topology;
    verify::arrangement(&model.flow, topology, &model.arrangement).expect("the probe conforms");
    let outer = &model.arrangement.contours[0];
    let inner = &model.arrangement.contours[1];
    assert_eq!(outer.side, inner.side, "both returns take the same flank");
    let entry =
        |loop_: &crate::topology::Loop| model.arrangement.rank[&Vertex::Junction(loop_.entry)];
    let tail =
        |loop_: &crate::topology::Loop| model.arrangement.rank[&Vertex::Junction(loop_.tail)];
    assert!(
        entry(&topology.loops[0]) > tail(&topology.loops[1])
            || entry(&topology.loops[1]) > tail(&topology.loops[0]),
        "the two returns should span rows that cannot cross"
    );

    // Move the inner return one step further out than the outer one. Nothing
    // crosses, and the outer return is now inside the body it leaves.
    let mut broken = model.arrangement.clone();
    broken.contours[1] = Contour {
        side: outer.side,
        column: outer.column,
        lane: outer.lane + 1,
    };
    let error = verify::arrangement(&model.flow, topology, &broken)
        .expect_err("the outer return no longer clears the nested one");
    assert!(error.contains("climbs inside its body"), "{error}");
}

mod reference;

/// Distributor routes may share rails, but all their case endpoints must
/// arrive on one row. Neither search may hide an enclosed exit below the tail.
#[test]
fn exits_between_repeating_cases_are_refused_by_both_procedures() {
    for routes in [
        ["repeat", "break", "repeat"].as_slice(),
        ["repeat", "end", "repeat"].as_slice(),
        ["repeat", "end", "end", "repeat"].as_slice(),
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
        ["repeat", "repeat", "end"].as_slice(),
        ["repeat", "repeat", "end", "end"].as_slice(),
        ["break", "break", "end"].as_slice(),
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

/// Both directions, against a procedure that assumes none of the walk's
/// transition rules, none of its reductions, and none of its arithmetic: a
/// topology the construction draws is one the reference finds a construction
/// for, and a topology it refuses has none. Positives are compared as well as
/// refusals; skipping the positives would leave the construction answering for
/// the flows it happens to draw.
///
/// A disagreement either way is a counterexample: a false refusal if the
/// reference finds a construction, an unsound acceptance if it does not.
/// The shapes every decision test walks: flat loop bodies of two, three and
/// four routes, and loops nested inside a loop, drawable and not.
pub(super) fn decision_cases() -> Vec<String> {
    let names = ["repeat", "break", "end"];
    let mut cases = Vec::new();
    for count in 2..=4 {
        for mut code in 0..names.len().pow(count) {
            let mut routes = Vec::new();
            for _ in 0..count {
                routes.push(names[code % names.len()]);
                code /= names.len();
            }
            cases.push(looping(&routes));
        }
    }
    for inner in [
        ["repeat", "break"].as_slice(),
        ["break", "repeat"].as_slice(),
        ["outer", "repeat"].as_slice(),
        ["repeat", "outer"].as_slice(),
        ["end", "repeat"].as_slice(),
        ["repeat", "break", "repeat"].as_slice(),
    ] {
        for other in names {
            cases.push(nested(&["inner", other], inner));
            cases.push(nested(&[other, "inner"], inner));
        }
    }
    cases.extend(diagram_shapes::question_shapes());
    cases
}

pub(super) fn small_decision_cases() -> Vec<String> {
    decision_cases()
        .into_iter()
        .take(36)
        .chain(diagram_shapes::question_shapes())
        .collect()
}

#[test]
fn the_construction_agrees_with_an_independent_procedure() {
    // Every flat body of two and three routes, positives as well as refusals
    // and nothing skipped. The wider domain, which takes long enough to keep
    // out of a default run, is the ignored test below.
    let names = ["repeat", "break", "end"];
    let mut cases = Vec::new();
    for count in 2..=3 {
        for mut code in 0..names.len().pow(count) {
            let mut routes = Vec::new();
            for _ in 0..count {
                routes.push(names[code % names.len()]);
                code /= names.len();
            }
            cases.push(looping(&routes));
        }
    }
    let counted = agree(&cases);
    assert!(
        counted.drawn == 29 && counted.refused == 3 && counted.rejected_earlier == 4,
        "the flat domain should preserve every known outcome: {counted:?}"
    );
}

/// The same comparison over the whole domain the plan declares, which takes
/// long enough to keep out of the default run.
///
/// Run it with:
/// `cargo test -p kaalang-model --lib the_declared_domain_agrees -- --ignored --nocapture`
#[test]
#[ignore = "exhaustive; run it with --ignored"]
fn the_declared_domain_agrees_with_the_independent_procedure() {
    let counted = agree(&declared_domain());
    println!("{counted:?}");
    assert!(
        counted.drawn == 325 && counted.refused == 113 && counted.rejected_earlier == 111,
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

/// Compares both procedures over `cases`, in both directions.
///
/// Against the deciding sweep, not against `construct`: the preferred search
/// would answer for the flows it happens to draw and hide whatever the sweep
/// did with them. A flow another rule rejects has no topology to arrange, and
/// realizability never had a say in it.
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
        let drawn = match super::sweep::search(&parts.flow, &parts.merges, &parts.topology) {
            Ok(built) => {
                verify::arrangement(&parts.flow, &parts.topology, &built).unwrap_or_else(
                    |reason| panic!("{source}\nthe sweep drew an invalid arrangement: {reason}"),
                );
                true
            }
            Err(super::sweep::Refusal::Impossible(_)) => false,
            Err(super::sweep::Refusal::Internal(reason)) => panic!("{source}\n{reason}"),
        };
        let flexible =
            match super::sweep::search_shape(&parts.flow, &parts.merges, &parts.topology, true) {
                Ok(_) => true,
                Err(super::sweep::Refusal::Impossible(_)) => false,
                Err(super::sweep::Refusal::Internal(reason)) => {
                    panic!("{source}\nflexible returns: {reason}")
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
    let model = model(&looping(&["repeat", "end"]));
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
            loop {
                loop {
                    #[question(\"Repeat?\")]
                    let (again, leave) = |flag| flag;
                    |leave| break;
                    #[action(\"Repeat.\")]
                    |again| ();
                }
            }
        }",
    );
    let [outer, inner] = model.topology.loops[..] else {
        panic!("the flow has two repeating loops");
    };
    let body = model.body_vertices(inner.header);
    assert!(body.contains(&Vertex::Junction(inner.tail)));
    assert!(!body.contains(&Vertex::Junction(outer.tail)));
}

/// A loop that never leaves its own body, written between two routes that meet
/// at the iteration tail and one that leaves the loop: the first and third
/// routes converge at the tail, the third and fourth after the loop, and the
/// second is a sibling of both groups whose lifelines end before either group's
/// vertex is placed.
///
/// The tail continues the first route's column, which is the selection's own
/// and stands left of the second branch however the rows are chosen. Holding
/// that earlier sibling left of the tail — the mirror image of the rule RFC
/// 0002 §8 states for a later sibling — refused this flow, and refused it only
/// once every interleaving had failed to number, so the answer never arrived
/// at all. The flow has a diagram: `kaalang-svg` draws it beside
/// `loop/behavior/diverging_middle_branch.rs`.
const DIVERGING_MIDDLE_BRANCH: &str = "fn diverging_middle_branch(mode: u8, stay: bool) -> u8 {
    loop {
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
        #[action(\"Advance through the first case.\")]
        |advance| {};
        |spin| loop {
            #[action(\"Spin.\")]
            || {};
        };
        #[question(\"Stay in the loop?\")]
        let (again, leave) = |decide, stay| stay;
        #[action(\"Advance through the third case.\")]
        |again| {};
        |leave| break;
        |leave_now| break;
    }
    #[action(\"Return the mode.\")]
    let end = |mode| mode;
}
";

#[test]
fn a_diverging_branch_between_partial_merges_is_drawn() {
    let parts = parts_of(DIVERGING_MIDDLE_BRANCH).expect("the flow passes the earlier rules");
    let built = match super::sweep::search(&parts.flow, &parts.merges, &parts.topology) {
        Ok(built) => built,
        Err(super::sweep::Refusal::Impossible(blocked)) => {
            panic!("the sweep refuses a drawable flow: {}", blocked.message)
        }
        Err(super::sweep::Refusal::Internal(reason)) => panic!("{reason}"),
    };
    verify::arrangement(&parts.flow, &parts.topology, &built)
        .expect("the sweep's arrangement conforms");
    model(DIVERGING_MIDDLE_BRANCH);
    assert!(
        reference::admissible(&parts.flow, &parts.topology),
        "the independent procedure should find a construction too"
    );
}

/// How the enumerated domain settles, which is the record section 6 of
/// `plans/diagram-realizability_3.md` asks for.
#[test]
#[ignore = "a measurement, not an assertion; run it with --ignored"]
fn record_every_generated_decision() {
    let (mut drawn, mut refused, mut semantic, mut internal) = (0, 0, 0, 0);
    for source in loop_shapes() {
        let function: syn::ItemFn = syn::parse_str(&source).expect("the probe parses");
        match crate::build(&function) {
            Ok(_) => drawn += 1,
            Err(error) if error.to_string().contains("internal kaalang") => internal += 1,
            Err(_) if parts_of(&source).is_none() => semantic += 1,
            Err(_) => {
                if refused == 0 {
                    println!("first refused topology:\n{source}");
                }
                refused += 1;
            }
        }
    }
    println!(
        "{} shapes: drawn {drawn}, refused {refused}, rejected earlier {semantic}, internal {internal}",
        drawn + refused + semantic + internal
    );
}

/// Every shape the earlier audit recorded reaches a decision, and none of them
/// an internal error.
///
/// The audit of `plans/diagram-realizability_3.md` section 2 listed eight
/// nested loop shapes that failed, five of them in the deciding search's
/// returns and three through the public build path. They are all inside the
/// generated domain, and this names them so a regression points at the row it
/// belongs to rather than at one of several thousand shapes.
#[test]
fn every_audit_shape_reaches_a_decision() {
    let cases: [(usize, &[&str], &[&str]); 8] = [
        (1, &["inner", "repeat", "repeat"], &["outer", "repeat"]),
        (2, &["break", "inner", "repeat"], &["outer", "repeat"]),
        (3, &["end", "inner", "repeat"], &["outer", "repeat"]),
        (4, &["break", "inner", "repeat"], &["end", "repeat"]),
        (5, &["end", "inner", "repeat"], &["end", "repeat"]),
        (6, &["break", "inner", "repeat"], &["outer", "break"]),
        (7, &["repeat", "end", "inner"], &["end", "break"]),
        (8, &["end", "inner", "repeat"], &["end", "break"]),
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
        // Whether each of these is drawable is settled by the independent
        // reference, not by the answer the construction gives here. What this
        // pins is that every one of them still reaches construction at all.
        parts_of(&source).expect("the audit shapes pass the earlier rules");
    }
}

/// A return may stand further out than its body needs, and the check says so.
///
/// The contour is a decision, not a suggestion: a renderer realizes the column
/// the arrangement recorded rather than the nearest position the body's boxes
/// suggest. This is the witness that pins the difference — the same topology
/// with its return one column beyond everything its body draws, still
/// conforming — and `kaalang-svg` renders it.
#[test]
fn a_return_may_stand_beyond_the_body_it_clears() {
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
        .expect("a return standing further out than its body still conforms");
}

/// The sole-arrival witness used by the SVG contour regression must also
/// pass the model's check before the renderer is held to its recorded column.
#[test]
fn a_return_with_one_arrival_may_stand_beyond_its_body() {
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

/// Four loops nested one inside the next, whose returns climb the same side in
/// four lanes. The outermost takes lane 3, which no fixture reaches.
pub(super) const FOUR_LANES: &str = "fn deep(mut step: usize) -> usize {
    loop {
        #[question(\"Leave the first?\")]
        let (stay_0, leave_0) = |&step| *step > 0;
        |leave_0| break;
        |stay_0| loop {
            #[question(\"Leave the second?\")]
            let (stay_1, leave_1) = |&step| *step > 1;
            |leave_1| break;
            |stay_1| loop {
                #[question(\"Leave the third?\")]
                let (stay_2, leave_2) = |&step| *step > 2;
                |leave_2| break;
                |stay_2| loop {
                    #[question(\"Leave the fourth?\")]
                    let (stay_3, leave_3) = |&step| *step > 3;
                    |leave_3| break;
                    #[action(\"Advance at the deepest level.\")]
                    |stay_3, &mut step| *step += 1;
                };
            };
        };
    }
    #[action(\"Return the step.\")]
    let end = |step| step;
}
";

/// A topology offers one contour lane per loop, and a witness can use the last
/// of them.
///
/// `contour_lanes` bounds the lanes at the loop count because only a return
/// climbs beside a column. Four mutually enclosing loops on one side is what
/// makes the bound tight, and lane 3 is the one no fixture and no generated
/// shape reaches. `kaalang-svg` draws this same witness.
#[test]
fn four_nested_returns_climb_four_lanes_on_one_side() {
    let model = model(FOUR_LANES);
    assert_eq!(model.topology.loops.len(), 4, "four nested loops");
    assert_eq!(
        verify::contour_lanes(&model.topology),
        4,
        "four loops offer four lanes"
    );

    let side = model.arrangement.contours[0].side;
    let mut deep = model.arrangement.clone();
    for (index, contour) in deep.contours.iter_mut().enumerate() {
        contour.side = side;
        contour.lane = deep_lane(index);
    }
    verify::arrangement(&model.flow, &model.topology, &deep)
        .expect("four returns in four lanes on one side conform");
    assert_eq!(
        deep.contours[0].lane, 3,
        "the outermost takes the last lane"
    );
}

/// Which lane the return of the `FOUR_LANES` loop at `index` climbs: the
/// outermost is furthest out, so the lanes run down with the nesting.
pub(super) const fn deep_lane(index: usize) -> usize {
    3 - index
}

/// A sibling moved into the columns its convergence group reserves is caught,
/// though nothing about the drawing crosses.
///
/// This is the rule no crossing check can stand in for. The group's area and
/// the sibling's own vertices are read off the topology, and the band is what
/// the group draws and the sibling does not, so the arrangement is held to a
/// range it did not choose.
#[test]
fn a_sibling_inside_a_reserved_footprint_is_caught() {
    let model = model(&looping(&["repeat", "repeat", "break"]));
    let reachable = super::regions::reachable(&model.topology);
    let block = super::regions::branchers(&model.flow)[0];
    let regions = super::regions::regions(&model.flow, &model.topology, &reachable, block);
    let group = regions
        .groups
        .iter()
        .find(|group| group.members.len() == 2)
        .expect("two of the three routes repeat, so they converge");
    let last = regions.branches.len() - 1;
    // Everything the sibling draws but its own head, whose column authored
    // branch order already fixes: what is left is the part only the reserved
    // columns hold to a side.
    let head = Vertex::Node(NodeId::Case {
        choice: block,
        branch: last,
    });
    let outside = regions
        .outside(group, last)
        .into_iter()
        .filter(|vertex| *vertex != head)
        .collect::<BTreeSet<_>>();
    assert!(
        !outside.is_empty(),
        "the break branch draws its own junction"
    );
    let inside = group
        .area
        .iter()
        .map(|vertex| model.arrangement.column[vertex])
        .max()
        .expect("the group draws something");

    let mut broken = model.arrangement.clone();
    for vertex in &outside {
        broken.column.insert(*vertex, inside);
    }
    // The corridors follow the columns they join, so the coverage rules still
    // hold and the reserved columns are what objects.
    for (index, wire) in model.topology.connections.iter().enumerate() {
        let departure = match wire.source {
            Source::Exit(exit) => {
                broken.column[&Vertex::Node(exit.node)] + broken.exit_offset[&exit]
            }
            Source::Junction(junction) => broken.column[&Vertex::Junction(junction)],
        };
        let arrival = broken.column[&wire.destination];
        broken.routes[index].departure = departure;
        broken.routes[index].arrival = arrival;
    }
    let reason = verify::arrangement(&model.flow, &model.topology, &broken)
        .expect_err("a sibling inside the reserved columns does not conform");
    assert!(
        reason.contains("its convergence group"),
        "the reserved columns should be the rule that objects: {reason}"
    );
}

#[test]
fn ordered_question_ports_cover_genuine_refusals_in_both_procedures() {
    let counted = agree(&diagram_shapes::question_shapes());
    println!("ordered questions: {counted:?}");
    assert_eq!(
        (counted.drawn, counted.refused, counted.rejected_earlier),
        (20, 6, 1)
    );
}

#[test]
fn an_end_between_breaks_cannot_escape_their_merge() {
    let counted = agree(&[looping(&["break", "end", "break", "repeat"])]);
    assert_eq!(counted.refused, 1);
}

#[test]
fn a_return_can_bend_outside_its_body() {
    let model = model(&looping(&["repeat", "repeat", "break"]));
    let mut bent = model.arrangement.clone();
    let contour = bent.contours[0];
    let delta = match contour.side {
        Side::Left => -1,
        Side::Right => 1,
    };
    let gap = bent.ranks / 2;
    let lane = bent.gap_lanes[gap];
    bent.gap_lanes[gap] += 1;
    bent.return_routes.insert(
        0,
        super::Route {
            departure: contour.column,
            arrival: contour.column + delta,
            runs: vec![super::Run {
                gap,
                lane,
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
    bent.return_routes.get_mut(&0).unwrap().runs[0].lane += 1;
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
        #[action("Return the value.")]
        let end = |value| value;
    }"#;
    let parts = parts_of(source).unwrap();
    let Ok(built) = super::sweep::search(&parts.flow, &parts.merges, &parts.topology) else {
        panic!("a serial flow has a diagram");
    };
    assert!(built.routes.iter().all(|route| route.runs.is_empty()));
    assert!(built.gap_lanes.iter().all(|&count| count == 0));
}
