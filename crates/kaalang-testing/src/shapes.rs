//! Source generators shared by the model and renderer tests. They share
//! inputs, not decision procedures or expected answers: agreement between two
//! crates means nothing when both read the same answer from one place.

use std::ops::RangeInclusive;

/// Four cycles nested one inside the next, whose back edges climb the same side
/// in four lanes. The outermost takes lane 3, which no fixture reaches.
pub const FOUR_LANES: &str = r#"#[kaa]
fn deep(mut step: usize) -> usize {
    #[kaa::cycle("Repeat the first cycle.")]
    let leave_0 = |step| loop {
        #[kaa::question("Leave the first?")]
        let (stay_0, leave_0) = |&step| *step > 0;
        #[kaa::cycle("Repeat the second cycle.")]
        let leave_1 = |stay_0| loop {
            #[kaa::question("Leave the second?")]
            let (stay_1, leave_1) = |&step| *step > 1;
            #[kaa::cycle("Repeat the third cycle.")]
            let leave_2 = |stay_1| loop {
                #[kaa::question("Leave the third?")]
                let (stay_2, leave_2) = |&step| *step > 2;
                #[kaa::cycle("Repeat the fourth cycle.")]
                let leave_3 = |stay_2| loop {
                    #[kaa::question("Leave the fourth?")]
                    let (stay_3, leave_3) = |&step| *step > 3;
                    #[kaa::action("Advance at the deepest level.")]
                    |stay_3, &mut step| *step += 1;
                    |stay_3| continue;
                };
                |leave_3| continue;
            };
            |leave_2| continue;
        };
        |leave_1| continue;
    };
    |leave_0, step| return step;
}
"#;

/// The outcomes a flat cycle body selects between.
const ROUTES: [&str; 3] = ["repeat", "leave", "finish"];

/// The same for a cycle nested in another, where `propagate` hands its result
/// to the enclosing cycle instead of completing only its own.
const NESTED_ROUTES: [&str; 4] = ["repeat", "leave", "propagate", "finish"];

/// Every body of `count` routes drawn from `names`, counted like an odometer
/// so the order is stable across the crates that pin their totals.
fn combinations(names: &[&'static str], count: u32) -> Vec<Vec<&'static str>> {
    (0..names.len().pow(count))
        .map(|mut code| {
            (0..count)
                .map(|_| {
                    let name = names[code % names.len()];
                    code /= names.len();
                    name
                })
                .collect()
        })
        .collect()
}

/// Every flat cycle body of `lengths` routes.
#[must_use]
pub fn flat_bodies(lengths: RangeInclusive<u32>) -> Vec<String> {
    lengths
        .flat_map(|count| combinations(&ROUTES, count))
        .map(|routes| cycle_routes(&routes))
        .collect()
}

/// A cycle whose body selects one route per named outcome, in that order.
/// `repeat` reaches the body's one `continue`, `leave` completes it with the
/// mode, and `finish` computes seven before completing the cycle.
///
/// # Panics
///
/// Panics on a route it does not generate.
#[must_use]
pub fn cycle_routes(routes: &[&str]) -> String {
    cycle_routes_with(routes, &selection(routes, "case_"))
}

/// [`cycle_routes`] opened by `choice` instead of the distributor it generates.
fn cycle_routes_with(routes: &[&str], choice: &str) -> String {
    let bodies = routes
        .iter()
        .enumerate()
        .map(|(index, route)| match *route {
            "leave" => format!(
                "        #[kaa::action(\"Keep the mode from case {index}.\")]\n        let completed = |case_{index}, mode| mode;"
            ),
            "finish" => format!(
                "        #[kaa::action(\"Finish from case {index}.\")]\n        let completed = |case_{index}| 7;"
            ),
            "repeat" => format!(
                "        #[kaa::action(\"Advance in case {index}.\")]\n        let again = |case_{index}| ();"
            ),
            other => panic!("unknown route {other} in {routes:?}"),
        })
        .collect::<Vec<_>>()
        .join("\n");
    let completes = routes.iter().any(|route| *route != "repeat");
    let repeats = routes.contains(&"repeat");
    cycle_flow(
        "Exercise the generated routes.",
        choice,
        &bodies,
        completes,
        repeats,
    )
}

/// The flow around one generated cycle body: the cycle's caption, the selection
/// that opens it, the transfer its repeating routes reach, and the return a
/// completing body needs. Repeating routes merge `again` before the one
/// `continue`; completing routes merge the declared output `completed`.
fn cycle_flow(
    caption: &str,
    selection: &str,
    bodies: &str,
    completes: bool,
    repeats: bool,
) -> String {
    let transfer = if repeats {
        "\n        |again| continue;"
    } else {
        ""
    };
    let (output, after) = if completes {
        ("let completed = ", "\n    |completed| return completed;")
    } else {
        ("", "")
    };
    format!(
        "fn probe(mode: u8) -> u8 {{\n    #[kaa::cycle(\"{caption}\")]\n    {output}|mode| loop {{\n{selection}\n{bodies}{transfer}\n    }};{after}\n}}\n"
    )
}

/// The outcomes of a cycle body with two declared outputs.
const ALTERNATIVE_ROUTES: [&str; 3] = ["repeat", "first", "second"];

/// Every flat body of `lengths` routes that exports both of two declared
/// outputs, `first` and `second`, and may repeat. Kept apart from the pinned
/// domains, which stay single-output.
#[must_use]
pub fn alternative_bodies(lengths: RangeInclusive<u32>) -> Vec<String> {
    lengths
        .flat_map(|count| combinations(&ALTERNATIVE_ROUTES, count))
        .filter(|routes| routes.contains(&"first") && routes.contains(&"second"))
        .map(|routes| alternative(&routes))
        .collect()
}

/// A cycle whose `first` and `second` routes export its two outputs, each
/// continued after the cycle before they merge at the return.
fn alternative(routes: &[&str]) -> String {
    let bodies = routes
        .iter()
        .enumerate()
        .map(|(index, route)| match *route {
            "repeat" => format!(
                "        #[kaa::action(\"Advance in case {index}.\")]\n        let again = |case_{index}| ();"
            ),
            output => format!(
                "        #[kaa::action(\"Export {output} from case {index}.\")]\n        let {output} = |case_{index}| {index}u8;"
            ),
        })
        .collect::<Vec<_>>()
        .join("\n");
    let transfer = if routes.contains(&"repeat") {
        "\n        |again| continue;"
    } else {
        ""
    };
    let selection = selection(routes, "case_");
    format!(
        "fn probe(mode: u8) -> u8 {{\n    #[kaa::cycle(\"Exercise the generated outputs.\")]\n    let (first, second) = |mode| loop {{\n{selection}\n{bodies}{transfer}\n    }};\n    #[kaa::action(\"Continue the first output.\")]\n    let result = |first| first;\n    #[kaa::action(\"Continue the second output.\")]\n    let result = |second| second + 100;\n    |result| return result;\n}}\n"
    )
}

/// A question that either bypasses a gated cycle of two or three outputs or
/// enters it. The bypass merges with every nonempty subset of the outputs, in
/// both question orders, with and without a repeating case after the exits.
/// The outputs left out meet that merge at the return.
#[must_use]
pub fn bypassed_cycles() -> Vec<String> {
    let mut shapes = Vec::new();
    for outputs in 2..=3 {
        for joined in 1..1usize << outputs {
            for bypass_first in [true, false] {
                for repeats in [false, true] {
                    shapes.push(bypassed_cycle(outputs, joined, bypass_first, repeats));
                }
            }
        }
    }
    shapes
}

/// One [`bypassed_cycles`] shape; bit `k` of `joined` merges output `k`.
fn bypassed_cycle(outputs: usize, joined: usize, bypass_first: bool, repeats: bool) -> String {
    let question = if bypass_first {
        "let (bypass, enter) = |flag| flag;"
    } else {
        "let (enter, bypass) = |flag| !flag;"
    };
    let mut routes = vec!["exit"; outputs];
    if repeats {
        routes.push("repeat");
    }
    let selection = selection(&routes, "out_");
    let transfer = if repeats {
        format!("\n        |out_{outputs}| continue;")
    } else {
        String::new()
    };
    let declared = (0..outputs)
        .map(|output| format!("out_{output}"))
        .collect::<Vec<_>>()
        .join(", ");
    let (merged, kept): (Vec<_>, Vec<_>) =
        (0..outputs).partition(|output| joined & (1 << output) != 0);
    let merged = merged
        .iter()
        .map(|output| {
            format!(
                "    #[kaa::action(\"Join output {output}.\")]\n    let joined = |out_{output}| {output}u8;\n"
            )
        })
        .collect::<Vec<_>>()
        .concat();
    let kept = kept
        .iter()
        .map(|output| {
            format!(
                "    #[kaa::action(\"Keep output {output}.\")]\n    let result = |out_{output}| {}u8;\n",
                output + 10
            )
        })
        .collect::<Vec<_>>()
        .concat();
    format!(
        "fn probe(flag: bool, mode: u8) -> u8 {{\n    #[kaa::question(\"Bypass the cycle?\")]\n    {question}\n    #[kaa::cycle(\"Select an output.\")]\n    let ({declared}) = |enter| loop {{\n{selection}{transfer}\n    }};\n    #[kaa::action(\"Bypass.\")]\n    let joined = |bypass| 100u8;\n{merged}    #[kaa::action(\"Use the joined value.\")]\n    let result = |joined| joined + 1;\n{kept}    |result| return result;\n}}\n"
    )
}

/// The choice that opens a cycle body, naming its outcomes `<prefix><index>`.
fn selection(routes: &[&str], prefix: &str) -> String {
    let cases = routes
        .iter()
        .enumerate()
        .map(|(index, route)| format!("        #[kaa::case(\"Case {prefix}{index} {route}.\")]"))
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
    format!(
        "        #[kaa::choice(\"Which {prefix} route?\")]\n{cases}\n        let ({outputs}) = |mode| match mode {{\n{arms}\n        }};"
    )
}

/// One cycle whose body selects `routes`, wrapped in an outer cycle whose other
/// branches take `propagate`. It uses the same names as [`cycle_routes`], plus
/// `propagate` for an inner result that the enclosing cycle explicitly handles.
/// The inner cycle repeats through `inner_again`: a nested local cannot reuse
/// the outer `again` it can see.
///
/// # Panics
///
/// Panics on a route it does not generate.
#[must_use]
pub fn nested(outer: &[&str], inner: &[&str]) -> String {
    let inner_selection = selection(inner, "i");
    let propagates = inner
        .iter()
        .any(|route| matches!(*route, "propagate" | "finish"));
    let inner_bodies = inner
        .iter()
        .enumerate()
        .map(|(index, route)| match (*route, propagates) {
            ("leave", false) => format!(
                "        #[kaa::action(\"Complete the inner cycle from i{index}.\")]\n        let again = |i{index}| ();"
            ),
            ("leave", true) => format!(
                "        #[kaa::action(\"Complete only the inner cycle from i{index}.\")]\n        let inner_value = |i{index}| None;"
            ),
            ("propagate", _) => format!(
                "        #[kaa::action(\"Complete the outer cycle from i{index}.\")]\n        let inner_value = |i{index}, mode| Some(mode);"
            ),
            ("finish", _) => format!(
                "        #[kaa::action(\"Finish from i{index}.\")]\n        let inner_value = |i{index}| Some(7);"
            ),
            ("repeat", _) => format!(
                "        #[kaa::action(\"Advance in i{index}.\")]\n        let inner_again = |i{index}| ();"
            ),
            (other, _) => panic!("unknown inner route {other} in {inner:?}"),
        })
        .collect::<Vec<_>>()
        .join("\n");
    let inner_completes = inner.iter().any(|route| *route != "repeat");
    let inner_transfer = if inner.contains(&"repeat") {
        "\n        |inner_again| continue;"
    } else {
        ""
    };
    // The inner cycle takes the outer route that selected it: a block sitting
    // where the outer branches are still separate has to belong to one of them.
    // Its completion repeats the outer cycle, so it declares `again` itself,
    // even where an earlier outer branch produces `again` too.
    let inner_cycle = |index: usize| {
        if propagates {
            format!(
                "        #[kaa::cycle(\"Exercise the generated inner routes.\")]\n        let inner_value = |o{index}| loop {{\n{inner_selection}\n{inner_bodies}{inner_transfer}\n        }};\n        #[kaa::question(\"Should the inner result complete the outer cycle?\")]\n        let (finish_outer, again) = |&inner_value| inner_value.is_some();\n        #[kaa::action(\"Extract the propagated inner result.\")]\n        let completed = |finish_outer, inner_value| inner_value.unwrap();"
            )
        } else if inner_completes {
            format!(
                "        #[kaa::cycle(\"Exercise the generated inner routes.\")]\n        let again = |o{index}| loop {{\n{inner_selection}\n{inner_bodies}{inner_transfer}\n        }};"
            )
        } else {
            format!(
                "        #[kaa::cycle(\"Exercise the generated inner routes.\")]\n        |o{index}| loop {{\n{inner_selection}\n{inner_bodies}{inner_transfer}\n        }};"
            )
        }
    };
    let outer_selection = selection(outer, "o");
    let outer_bodies = outer
        .iter()
        .enumerate()
        .map(|(index, route)| match *route {
            "leave" => format!(
                "        #[kaa::action(\"Keep the mode from o{index}.\")]\n        let completed = |o{index}, mode| mode;"
            ),
            "finish" => format!(
                "        #[kaa::action(\"Finish from o{index}.\")]\n        let completed = |o{index}| 7;"
            ),
            "inner" => inner_cycle(index),
            "repeat" => format!(
                "        #[kaa::action(\"Advance in o{index}.\")]\n        let again = |o{index}| ();"
            ),
            other => panic!("unknown outer route {other} in {outer:?}"),
        })
        .collect::<Vec<_>>()
        .join("\n");
    let completes = outer
        .iter()
        .any(|route| matches!(*route, "leave" | "finish"))
        || outer.contains(&"inner") && propagates;
    let repeats = outer.contains(&"repeat") || outer.contains(&"inner") && inner_completes;
    cycle_flow(
        "Exercise the generated outer routes.",
        &outer_selection,
        &outer_bodies,
        completes,
        repeats,
    )
}

/// Exhaustive comparison domain: flat bodies of 2–4 `repeat/leave/finish` routes,
/// and three-route outer bodies with two-route `repeat/leave/propagate/finish` cycles.
#[must_use]
pub fn declared_domain() -> Vec<String> {
    let mut cases = flat_bodies(2..=4);
    for position in 0..3 {
        for first in ROUTES {
            for second in ROUTES {
                let mut outer = vec![first, second];
                outer.insert(position, "inner");
                for left in NESTED_ROUTES {
                    for right in NESTED_ROUTES {
                        cases.push(nested(&outer, &[left, right]));
                    }
                }
            }
        }
    }
    cases
}

/// Every generated cycle shape, flat and nested. Both the model and the SVG
/// tests chain [`question_shapes`] onto this for the corpus they count.
#[must_use]
pub fn cycle_shapes() -> Vec<String> {
    let mut shapes = declared_domain();
    shapes.extend(flat_bodies(5..=5));
    for count in 2..=4 {
        for inner in combinations(&NESTED_ROUTES, count) {
            for position in 0..2 {
                for other in ROUTES {
                    let mut outer = vec![other; 2];
                    outer[position] = "inner";
                    shapes.push(nested(&outer, &inner));
                }
            }
        }
    }
    shapes
}

/// The same outcomes carried by ordered question ports, rather than a shared
/// distributor exit. Both forms must respect authored branch order, while
/// their different ports and endpoints exercise different constructions.
#[must_use]
pub fn question_shapes() -> Vec<String> {
    const QUESTIONS: &str = r#"        #[kaa::question("Take the first route?")]
        let (case_0, other) = |mode| mode == 0;
        #[kaa::question("Take the second route?")]
        let (case_1, case_2) = |other, mode| mode == 1;"#;
    combinations(&ROUTES, 3)
        .iter()
        .map(|routes| cycle_routes_with(routes, QUESTIONS))
        .collect()
}

/// Staged graphs with self, forward, backward, terminal, and diverging routes.
/// The variants put a cycle in preparation, in a stage, or neither; the last
/// two shapes exercise alternative outputs from single and nested stage cycles.
#[must_use]
pub fn staged_shapes() -> Vec<String> {
    const GRAPH: &str = r#"
        #[kaa]
        fn probe(INPUT: u8) -> u8 {
            PREPARATION

            #[kaa::stage("First.")]
            let (first, second, finish) = |first| {
                FIRST_VALUE
                #[kaa::choice("Choose the first route.")]
                #[kaa::case("Stay.")]
                #[kaa::case("Advance.")]
                #[kaa::case("Finish.")]
                let (stay, advance, done) = |current| match current % 3 {
                    0 => (),
                    1 => (),
                    _ => (),
                };
                #[kaa::action("Stay at first.")]
                let first = |stay, current| current;
                #[kaa::action("Advance to second.")]
                let second = |advance, current| current;
                #[kaa::action("Finish from first.")]
                let finish = |done, current| current;
            };

            #[kaa::stage("Second.")]
            let (first, second, finish) = |second| {
                #[kaa::action("Read the second entry.")]
                let current = |second| second;
                #[kaa::choice("Choose the second route.")]
                #[kaa::case("Go back.")]
                #[kaa::case("Stay.")]
                #[kaa::case("Finish.")]
                #[kaa::case("Diverge.")]
                let (back, stay, done, diverge) = |current| match current % 4 {
                    0 => (),
                    1 => (),
                    2 => (),
                    _ => (),
                };
                #[kaa::action("Return to first.")]
                let first = |back, current| current;
                #[kaa::action("Stay at second.")]
                let second = |stay, current| current;
                #[kaa::action("Finish from second.")]
                let finish = |done, current| current;
                #[kaa::cycle("Diverge in second.")]
                |diverge| loop { continue; };
            };

            #[kaa::stage("Return.")]
            |finish| { |finish| return finish; };
        }
    "#;
    let shape = |input: &str, preparation: &str, first_value: &str| {
        GRAPH
            .replace("INPUT", input)
            .replace("PREPARATION", preparation)
            .replace("FIRST_VALUE", first_value)
    };
    let read = "#[kaa::action(\"Read the first entry.\")] let current = |first| first;";
    vec![
        shape("first", "", read),
        shape(
            "seed",
            "#[kaa::cycle(\"Prepare without a gate.\")] let first = loop { #[kaa::action(\"Copy the seed.\")] let first = |seed| seed; };",
            read,
        ),
        shape(
            "first",
            "",
            "#[kaa::cycle(\"Read or repeat.\")] let current = |first| loop { #[kaa::question(\"Repeat?\")] let (retry, ready) = |first| first == 255; |retry| continue; #[kaa::action(\"Use the entry.\")] let current = |ready, first| first; };",
        ),
        include_str!("../../kaalang/tests/flow/stage/stage_cycle_alternative_outputs.rs")
            .to_owned(),
        include_str!("../../kaalang/tests/flow/stage/nested_stage_alternative_outputs.rs")
            .to_owned(),
    ]
}
