use std::collections::BTreeSet;

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

/// A loop whose body selects one route per named outcome, in that order.
/// `repeat` falls through to the end of the body, `break` leaves the loop, and
/// `end` finishes the flow from inside it.
pub(super) fn looping(routes: &[&str]) -> String {
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
    let function: syn::ItemFn = syn::parse_str(source).ok()?;
    let mut flow = crate::parse::flow(&function).ok()?;
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
    // The seven shapes with no diagram, each a route that leaves the body
    // between two that repeat it, or a route that repeats between two that
    // leave: the middle one is enclosed either way. `repeat, break, repeat`
    // and `repeat, end, repeat` are the two the compile-fail fixtures pin.
    let refused = [
        ["repeat", "break", "repeat"],
        ["repeat", "end", "repeat"],
        ["break", "repeat", "break"],
        ["break", "repeat", "end"],
        ["break", "end", "break"],
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

/// End cannot escape the two repeating routes that enclose it while staying
/// below the loop's iteration tail.
#[test]
fn a_terminal_route_between_two_repeats_cannot_leave_end_last() {
    let source = looping(&["repeat", "end", "repeat"]);
    let function = syn::parse_str(&source).expect("the probe parses");
    let error = crate::build(&function)
        .err()
        .expect("end cannot be placed last");
    assert!(
        error.to_string().contains("Reorder the branches"),
        "{error}"
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
    let wordy = plain.replace(
        "Case ",
        "Разверните шаг и продолжайте, пока счётчик не дойдёт до конца — case ",
    );
    assert_ne!(plain, wordy, "the probe should carry descriptions");
    let first = model(&plain).arrangement;
    let second = model(&wordy).arrangement;
    assert_eq!(first.rank, second.rank);
    assert_eq!(first.ranks, second.ranks);
    assert_eq!(first.column, second.column);
    assert_eq!(first.exit_offset, second.exit_offset);
    assert_eq!(first.routes, second.routes);
    assert_eq!(first.gap_lanes, second.gap_lanes);
    assert_eq!(first.contours, second.contours);
    // Nothing else is recorded, so nothing else can differ.
    let Arrangement {
        rank: _,
        ranks: _,
        column: _,
        exit_offset: _,
        routes: _,
        gap_lanes: _,
        contours: _,
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
    // Nothing else is recorded, so nothing else can differ.
    let Arrangement {
        rank: _,
        ranks: _,
        column: _,
        exit_offset: _,
        routes: _,
        gap_lanes: _,
        contours: _,
    } = first;
}

/// A topology with no conforming diagram is an authored-flow error, reported
/// at the vertex the sweep could not reach once it had visited every state.
#[test]
fn an_impossible_topology_rejects_the_model() {
    let function: syn::ItemFn =
        syn::parse_str(&looping(&["repeat", "break", "repeat"])).expect("the probe parses");
    let error = crate::build(&function)
        .err()
        .expect("a break enclosed by two repeats has no diagram");
    assert_eq!(
        error.to_string(),
        "could not construct a diagram under RFC 0002: nothing draws the iteration tail of \
         this loop: however the routes above it are arranged, another route lies between two \
         that meet there. Reorder the branches so the routes that meet sit side by side"
    );
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
            Err(blocked) => panic!(
                "{name}: the sweep refuses a drawable flow: {}",
                blocked.message
            ),
            Ok(built) => {
                verify::arrangement(&model.flow, &model.topology, &built).unwrap_or_else(
                    |reason| panic!("{name}: the sweep drew an invalid arrangement: {reason}"),
                );
            }
        }
    }
    assert!(checked > 100, "the corpus should be the whole tree");
}

/// One loop whose body selects `routes`, wrapped in an outer loop whose other
/// branches take `outer`. `outer` uses the same names as `looping`, plus
/// `outer` for a labelled break out of the enclosing loop.
fn nested(outer: &[&str], inner: &[&str]) -> String {
    let body = |routes: &[&str], prefix: &str, inner: &str| {
        let cases = routes
            .iter()
            .enumerate()
            .map(|(index, route)| format!("        #[case(\"Case {prefix}{index} {route}.\")]"))
            .collect::<Vec<_>>()
            .join("\n");
        let outputs = (0..routes.len())
            .map(|index| format!("{prefix}{index}"))
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
                "break" => format!("        |{prefix}{index}| break;"),
                "outer" => format!("        |{prefix}{index}| break 'outer;"),
                "end" => format!(
                    "        #[action(\"Finish from {prefix}{index}.\")]\n        let end = |{prefix}{index}| 7;"
                ),
                "inner" => format!("        |{prefix}{index}| loop {{\n{inner}\n        }};"),
                _ => format!(
                    "        #[action(\"Advance in {prefix}{index}.\")]\n        |{prefix}{index}| ();"
                ),
            })
            .collect::<Vec<_>>()
            .join("\n");
        format!(
            "        #[choice(\"Which {prefix} route?\")]\n{cases}\n        let ({outputs}) = |mode| match mode {{\n{arms}\n        }};\n{bodies}"
        )
    };
    let inner_body = body(inner, "i", "");
    let outer_body = body(outer, "o", &inner_body);
    let after = if outer.contains(&"break") || inner.contains(&"outer") {
        "\n    #[action(\"Return the mode.\")]\n    let end = |mode| mode;"
    } else {
        ""
    };
    format!("fn probe(mode: u8) -> u8 {{\n    'outer: loop {{\n{outer_body}\n    }}{after}\n}}\n")
}

/// No generated loop shape, flat or nested, reaches an internal construction
/// error: every one of them is drawn, or refused for a reason the author can
/// act on, or rejected by a semantic rule before construction.
///
/// An internal error means the search returned an arrangement its own check
/// rejects. A fixture corpus cannot find those — the shapes that produce them
/// are the ones nobody writes by hand.
/// Every flat and nested loop shape of two to five routes, as source.
fn loop_shapes() -> Vec<String> {
    let names = ["repeat", "break", "end"];
    let mut shapes = Vec::new();
    for count in 2..=5 {
        for mut code in 0..names.len().pow(count) {
            let mut routes = Vec::new();
            for _ in 0..count {
                routes.push(names[code % names.len()]);
                code /= names.len();
            }
            shapes.push(looping(&routes));
        }
    }
    let nested_names = ["repeat", "break", "outer", "end"];
    for count in 2..=4 {
        for mut code in 0..nested_names.len().pow(count) {
            let mut inner = Vec::new();
            for _ in 0..count {
                inner.push(nested_names[code % nested_names.len()]);
                code /= nested_names.len();
            }
            for position in 0..2 {
                for other in names {
                    let mut outer = vec![other; 2];
                    outer[position] = "inner";
                    shapes.push(nested(&outer, &inner));
                }
            }
        }
    }
    shapes
}

#[test]
fn no_generated_loop_shape_reaches_an_internal_error() {
    let shapes = loop_shapes();
    let mut drawn = 0;
    for source in &shapes {
        let function: syn::ItemFn = syn::parse_str(source).expect("the probe parses");
        match crate::build(&function) {
            Ok(model) => {
                drawn += 1;
                verify::arrangement(&model.flow, &model.topology, &model.arrangement)
                    .unwrap_or_else(|reason| panic!("{source}\n{reason}"));
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
            Err(blocked) => panic!(
                "{source}\nthe sweep refuses a drawable flow: {}",
                blocked.message
            ),
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

/// An independent decision procedure over a larger space than the production
/// walk, deciding only through the shared check.
///
/// It calls none of `sweep`'s transition rules and repeats none of its
/// reductions. Every vertex that can be placed is tried, not just the first;
/// the lifelines a vertex opens are tried in every order; and the column it
/// continues is tried over every route arriving there, rather than being
/// picked by a rule. A wrong rule in any of those shows up as a flow this
/// procedure draws and the production one refuses.
///
/// Two things it does not vary, because the shared crossing rule forces them
/// rather than a rule choosing them, and because varying them is what puts the
/// space out of reach:
///
/// - the lifelines a vertex ends stand side by side. They meet on its rail,
///   which spans them, so a live lifeline between two of them crosses that
///   rail. This procedure works that out for itself rather than asking
///   `sweep`.
/// - what a vertex opens takes the place of what it ended. Its outgoing routes
///   leave its own box sideways before they descend, so a live lifeline
///   between its column and one of theirs is crossed just the same.
///
/// It also shares the expansion — the arithmetic turning a sequence of choices
/// into ranks, columns and corridors — and the check. The check is independent
/// of both, and it is what catches a mistake in that arithmetic.
///
/// Bounds: every order of placing the vertices, every order of the lifelines
/// each one opens, and every arriving column each one may continue, with no
/// memory of states already refused — a state's completability depends on the
/// whole prefix here, not only on the frontier, so there is nothing sound to
/// remember. That is exponential, and the cases below are sized for it.
mod reference {
    use std::collections::BTreeMap;

    use super::verify;
    use super::{Flow, Topology};
    use crate::construct::sweep::{Lifeline, Step};
    use crate::topology::Vertex;

    struct Space<'a> {
        flow: &'a Flow,
        topology: &'a Topology,
        tail_of: Vec<Option<usize>>,
        arrivals: Vec<Vec<usize>>,
        departures: Vec<Vec<(bool, Vec<usize>)>>,
        predecessors: Vec<Vec<usize>>,
        entry_of: BTreeMap<usize, usize>,
        /// How many states the walk has entered, so a case that grows out of
        /// hand fails loudly instead of hanging.
        visited: usize,
        found: Option<crate::Arrangement>,
    }

    /// Whether any construction of this topology passes the shared check.
    pub(super) fn admissible(flow: &Flow, topology: &Topology) -> bool {
        witness(flow, topology).is_some()
    }

    /// The first construction the space yields that the shared check accepts.
    fn witness(flow: &Flow, topology: &Topology) -> Option<crate::Arrangement> {
        let index = crate::construct::sweep::index(flow, topology);
        let entry_of = topology
            .loops
            .iter()
            .enumerate()
            .map(|(index, loop_)| {
                (
                    topology
                        .vertices
                        .binary_search(&Vertex::Junction(loop_.entry))
                        .expect("a loop owns an entry"),
                    index,
                )
            })
            .collect();
        let mut space = Space {
            flow,
            topology,
            tail_of: index.tail_of,
            arrivals: index.arrivals,
            departures: index.departures,
            predecessors: index.predecessors,
            entry_of,
            visited: 0,
            found: None,
        };
        let mut placed = vec![false; topology.vertices.len()];
        let mut frontier = Vec::new();
        let mut steps = Vec::new();
        space.walk(&mut placed, &mut frontier, &mut steps);
        space.found
    }

    impl Space<'_> {
        fn walk(
            &mut self,
            placed: &mut Vec<bool>,
            frontier: &mut Vec<Lifeline>,
            steps: &mut Vec<Step>,
        ) -> bool {
            if placed.iter().all(|placed| *placed) {
                let Some(built) = crate::construct::sweep::expand(self.flow, self.topology, steps)
                else {
                    return false;
                };
                if verify::arrangement(self.flow, self.topology, &built).is_err() {
                    return false;
                }
                self.found = Some(built);
                return true;
            }
            self.visited += 1;
            assert!(
                self.visited < 200_000_000,
                "the reference space grew too large"
            );
            for vertex in 0..placed.len() {
                if placed[vertex] || self.predecessors[vertex].iter().any(|&up| !placed[up]) {
                    continue;
                }
                let mut ending = self.arrivals[vertex]
                    .iter()
                    .map(|&wire| Lifeline::Wire(wire))
                    .collect::<Vec<_>>();
                if let Some(loop_) = self.tail_of[vertex] {
                    ending.push(Lifeline::Return(loop_));
                }
                let Some(position) = side_by_side(frontier, &ending) else {
                    continue;
                };
                let consumed = frontier[position..position + ending.len()].to_vec();
                // The cases of one choice keep their authored order
                // (RFC 0002 §8); everything else a vertex opens may go either
                // way round, the return of a loop entry included.
                let mut opened = Vec::new();
                for (fixed, connections) in &self.departures[vertex] {
                    opened.push((
                        *fixed,
                        connections
                            .iter()
                            .map(|&wire| Lifeline::Wire(wire))
                            .collect::<Vec<_>>(),
                    ));
                }
                if let Some(&loop_) = self.entry_of.get(&vertex) {
                    opened.push((false, vec![Lifeline::Return(loop_)]));
                }
                for emitted in emissions(&opened) {
                    for continues in continuations(&consumed) {
                        let step = Step {
                            vertex,
                            consumed: consumed.clone(),
                            position,
                            emitted: emitted.clone(),
                            continues,
                        };
                        let kept = frontier.clone();
                        crate::construct::sweep::replay(frontier, &step);
                        placed[vertex] = true;
                        steps.push(step);
                        let found = self.walk(placed, frontier, steps);
                        steps.pop();
                        placed[vertex] = false;
                        *frontier = kept;
                        if found {
                            return true;
                        }
                    }
                }
            }
            false
        }
    }

    /// Where `ending` stands in the frontier, when its lifelines are all there
    /// and stand together. A vertex whose arrivals are apart has a live
    /// lifeline crossing its rail, so it cannot be drawn here at all.
    fn side_by_side(frontier: &[Lifeline], ending: &[Lifeline]) -> Option<usize> {
        if ending.is_empty() {
            return frontier.is_empty().then_some(0);
        }
        let first = frontier
            .iter()
            .position(|lifeline| ending.contains(lifeline))?;
        let run = frontier.get(first..first + ending.len())?;
        run.iter()
            .all(|lifeline| ending.contains(lifeline))
            .then_some(first)
    }

    /// Every order the groups a vertex opens may be laid out in: a fixed
    /// group keeps its order, a free one takes any, and the groups themselves
    /// may be interleaved in any order.
    fn emissions(groups: &[(bool, Vec<Lifeline>)]) -> Vec<Vec<Lifeline>> {
        let mut laid = vec![Vec::new()];
        for (fixed, group) in groups {
            let choices = if *fixed {
                vec![group.clone()]
            } else {
                orders(group)
            };
            laid = laid
                .into_iter()
                .flat_map(|start| {
                    choices.clone().into_iter().flat_map(move |group| {
                        // The group may sit anywhere among what is already
                        // laid out, not only after it.
                        (0..=start.len())
                            .map(|at| {
                                let mut next = start.clone();
                                next.splice(at..at, group.iter().copied());
                                next
                            })
                            .collect::<Vec<_>>()
                    })
                })
                .collect();
        }
        laid
    }

    /// Every order a handful of lifelines may be opened in.
    fn orders(items: &[Lifeline]) -> Vec<Vec<Lifeline>> {
        if items.len() <= 1 {
            return vec![items.to_vec()];
        }
        (0..items.len())
            .flat_map(|taken| {
                let mut rest = items.to_vec();
                let first = rest.remove(taken);
                orders(&rest).into_iter().map(move |mut order| {
                    order.insert(0, first);
                    order
                })
            })
            .collect()
    }

    /// Every column a vertex may continue: any arriving route, or none.
    fn continuations(consumed: &[Lifeline]) -> Vec<Option<Lifeline>> {
        std::iter::once(None)
            .chain(
                consumed
                    .iter()
                    .copied()
                    .filter(|lifeline| matches!(lifeline, Lifeline::Wire(_)))
                    .map(Some),
            )
            .collect()
    }
}

/// Both directions, against a procedure that assumes none of the walk's
/// transition rules: a topology the construction draws is one the reference
/// finds a construction for, and a topology it refuses has none.
///
/// The cases are chosen small enough for the reference to exhaust, and they
/// include refused shapes and nested loops, not only drawable flat ones. A
/// disagreement either way is a counterexample: a false refusal if the
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
    cases
}

#[test]
fn the_construction_agrees_with_an_independent_procedure() {
    let mut checked = (0, 0);
    // Every two-route body, and every three-route body the construction
    // refuses. A refusal is where agreement matters: the construction draws
    // its positives itself, so the reference only has to confirm that nothing
    // it turns down can be drawn. The reference keeps no reductions, and a
    // three-route body it has to search for a positive costs tens of seconds.
    let names = ["repeat", "break", "end"];
    let mut cases = Vec::new();
    for count in 2..=3 {
        for mut code in 0..names.len().pow(count) {
            let mut routes = Vec::new();
            for _ in 0..count {
                routes.push(names[code % names.len()]);
                code /= names.len();
            }
            cases.push((count, looping(&routes)));
        }
    }
    for (count, source) in &cases {
        // A flow another rule rejects has no topology to arrange, and
        // realizability never had a say in it.
        let Some(parts) = parts_of(source) else {
            continue;
        };
        // Against the deciding sweep, not against `construct`: the preferred
        // search would answer for the flows it happens to draw and hide
        // whatever the sweep did with them.
        let drawn = super::sweep::search(&parts.flow, &parts.merges, &parts.topology)
            .is_ok_and(|built| verify::arrangement(&parts.flow, &parts.topology, &built).is_ok());
        if drawn {
            if *count > 2 {
                continue;
            }
            checked.0 += 1;
        } else {
            checked.1 += 1;
        }
        assert_eq!(
            reference::admissible(&parts.flow, &parts.topology),
            drawn,
            "{source}"
        );
    }
    assert!(
        checked.0 >= 5 && checked.1 >= 1,
        "the cases should cover both outcomes: {checked:?}"
    );
}

/// The routes a choice's distributor fans out descend in the order of the
/// branches they carry, and `verify::distributor_order` is what says so.
///
/// Where each case sits one rank below its choice the rule adds nothing: the
/// route has one gap to turn in, so the column it descends in *is* the case's,
/// and the authored-order clause of `branch_columns` already holds those in
/// order. It is only where a case sits further down that a route can descend
/// in one column and cross to another before it arrives — and then nothing
/// crosses, because the routes share the distributor's own run. That is the
/// construction the independent reference found for `repeat, break, repeat`,
/// and this builds it directly.
#[test]
fn the_check_holds_a_distributor_to_the_authored_order() {
    // The sweep's arrangement, not the preferred search's: the preferred one
    // puts every case one rank below its choice, where the rule adds nothing.
    let source = looping(&["repeat", "repeat", "break"]);
    let parts = parts_of(&source).expect("the probe projects");
    let (flow, topology) = (&parts.flow, &parts.topology);
    let Ok(built) = super::sweep::search(flow, &parts.merges, topology) else {
        panic!("the probe has a diagram")
    };
    let valid = &built;
    verify::arrangement(flow, topology, valid).expect("the arrangement conforms");

    let exit = topology
        .exits
        .iter()
        .find(|exit| super::regions::carries_branches(topology, exit.id))
        .expect("a choice owns a distributor");
    // The first case whose route has a gap to spare below the one it turns in.
    let (index, turn) = topology
        .connections
        .iter()
        .enumerate()
        .filter(|(_, wire)| wire.source == Source::Exit(exit.id))
        .find_map(|(index, wire)| {
            let route = &valid.routes[index];
            let run = route.runs.first()?;
            let arrives = valid.rank[&wire.destination];
            (arrives > run.gap + 1).then_some((index, run.gap))
        })
        .expect("a case sits more than one rank below its choice");

    // Send it down beyond the case written after it, then across to its own
    // column in the gap below. It crosses nothing: the run it leaves by is the
    // distributor's, which every case route shares.
    let mut beyond = valid.clone();
    let past = valid
        .column
        .values()
        .copied()
        .max()
        .expect("the arrangement has columns")
        + 1;
    if let Some(lanes) = beyond.gap_lanes.get_mut(turn + 1) {
        *lanes = (*lanes).max(1);
    }
    let route = &mut beyond.routes[index];
    let home = route.arrival;
    route.runs = vec![
        super::Run {
            gap: turn,
            enter: route.departure,
            exit: past,
            lane: 0,
        },
        super::Run {
            gap: turn + 1,
            enter: past,
            exit: home,
            lane: 0,
        },
    ];

    let refused = verify::arrangement(flow, topology, &beyond)
        .expect_err("a distributor out of order has no diagram");
    assert!(
        refused.contains("not the authored one"),
        "the distributor rule should be what refuses it: {refused}"
    );
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
