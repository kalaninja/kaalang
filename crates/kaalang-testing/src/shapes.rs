//! Source generators shared by the model and renderer tests. They share
//! inputs, not decision procedures or expected answers: agreement between two
//! crates means nothing when both read the same answer from one place.

use std::ops::RangeInclusive;

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
        .map(|routes| looping(&routes))
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
pub fn looping(routes: &[&str]) -> String {
    looping_with(routes, &selection(routes, "case_"))
}

/// [`looping`] opened by `choice` instead of the distributor it generates.
fn looping_with(routes: &[&str], choice: &str) -> String {
    let bodies = routes
        .iter()
        .enumerate()
        .map(|(index, route)| match *route {
            "leave" => format!(
                "        #[action(\"Keep the mode from case {index}.\")]\n        let completed = |case_{index}, mode| mode;"
            ),
            "finish" => format!(
                "        #[action(\"Finish from case {index}.\")]\n        let completed = |case_{index}| 7;"
            ),
            "repeat" => format!(
                "        #[action(\"Advance in case {index}.\")]\n        let again = |case_{index}| ();"
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
        "fn probe(mode: u8) -> u8 {{\n    #[cycle(\"{caption}\")]\n    {output}|mode| {{\n{selection}\n{bodies}{transfer}\n    }};{after}\n}}\n"
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
                "        #[action(\"Advance in case {index}.\")]\n        let again = |case_{index}| ();"
            ),
            output => format!(
                "        #[action(\"Export {output} from case {index}.\")]\n        let {output} = |case_{index}| {index}u8;"
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
        "fn probe(mode: u8) -> u8 {{\n    #[cycle(\"Exercise the generated outputs.\")]\n    let (first, second) = |mode| {{\n{selection}\n{bodies}{transfer}\n    }};\n    #[action(\"Continue the first output.\")]\n    let result = |first| first;\n    #[action(\"Continue the second output.\")]\n    let result = |second| second + 100;\n    |result| return result;\n}}\n"
    )
}

/// The choice that opens a cycle body, naming its outcomes `<prefix><index>`.
fn selection(routes: &[&str], prefix: &str) -> String {
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
    format!(
        "        #[choice(\"Which {prefix} route?\")]\n{cases}\n        let ({outputs}) = |mode| match mode {{\n{arms}\n        }};"
    )
}

/// One cycle whose body selects `routes`, wrapped in an outer cycle whose other
/// branches take `propagate`. It uses the same names as `looping`, plus
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
                "        #[action(\"Complete the inner cycle from i{index}.\")]\n        let again = |i{index}| ();"
            ),
            ("leave", true) => format!(
                "        #[action(\"Complete only the inner cycle from i{index}.\")]\n        let inner_value = |i{index}| None;"
            ),
            ("propagate", _) => format!(
                "        #[action(\"Complete the outer cycle from i{index}.\")]\n        let inner_value = |i{index}, mode| Some(mode);"
            ),
            ("finish", _) => format!(
                "        #[action(\"Finish from i{index}.\")]\n        let inner_value = |i{index}| Some(7);"
            ),
            ("repeat", _) => format!(
                "        #[action(\"Advance in i{index}.\")]\n        let inner_again = |i{index}| ();"
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
                "        #[cycle(\"Exercise the generated inner routes.\")]\n        let inner_value = |o{index}| {{\n{inner_selection}\n{inner_bodies}{inner_transfer}\n        }};\n        #[question(\"Should the inner result complete the outer cycle?\")]\n        let (finish_outer, again) = |&inner_value| inner_value.is_some();\n        #[action(\"Extract the propagated inner result.\")]\n        let completed = |finish_outer, inner_value| inner_value.unwrap();"
            )
        } else if inner_completes {
            format!(
                "        #[cycle(\"Exercise the generated inner routes.\")]\n        let again = |o{index}| {{\n{inner_selection}\n{inner_bodies}{inner_transfer}\n        }};"
            )
        } else {
            format!(
                "        #[cycle(\"Exercise the generated inner routes.\")]\n        |o{index}| {{\n{inner_selection}\n{inner_bodies}{inner_transfer}\n        }};"
            )
        }
    };
    let outer_selection = selection(outer, "o");
    let outer_bodies = outer
        .iter()
        .enumerate()
        .map(|(index, route)| match *route {
            "leave" => format!(
                "        #[action(\"Keep the mode from o{index}.\")]\n        let completed = |o{index}, mode| mode;"
            ),
            "finish" => format!(
                "        #[action(\"Finish from o{index}.\")]\n        let completed = |o{index}| 7;"
            ),
            "inner" => inner_cycle(index),
            "repeat" => format!(
                "        #[action(\"Advance in o{index}.\")]\n        let again = |o{index}| ();"
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
pub fn loop_shapes() -> Vec<String> {
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
///
#[must_use]
pub fn question_shapes() -> Vec<String> {
    const QUESTIONS: &str = r#"        #[question("Take the first route?")]
        let (case_0, other) = |mode| mode == 0;
        #[question("Take the second route?")]
        let (case_1, case_2) = |other, mode| mode == 1;"#;
    combinations(&ROUTES, 3)
        .iter()
        .map(|routes| looping_with(routes, QUESTIONS))
        .collect()
}
