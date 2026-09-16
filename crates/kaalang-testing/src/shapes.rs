//! Source generators shared by the model and renderer tests. They share
//! inputs, not decision procedures or expected answers: agreement between two
//! crates means nothing when both read the same answer from one place.

use std::ops::RangeInclusive;

/// The outcomes a flat cycle body selects between.
const ROUTES: [&str; 3] = ["repeat", "break", "finish"];

/// The same for a cycle nested in another, where `propagate` hands its result
/// to the enclosing cycle instead of completing only its own.
const NESTED_ROUTES: [&str; 4] = ["repeat", "break", "propagate", "finish"];

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
/// `repeat` falls through to the end of the body, `break` returns the mode, and
/// `finish` computes seven before completing the cycle.
///
/// # Panics
///
/// Panics on a route it does not generate. The arms below fall through to
/// `repeat`, so a misspelled name would quietly drop its case instead.
#[must_use]
pub fn looping(routes: &[&str]) -> String {
    assert!(
        routes.iter().all(|route| ROUTES.contains(route)),
        "unknown route in {routes:?}"
    );
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
            "break" => format!(
                "        #[action(\"Keep the mode from case {index}.\")]\n        let completed = |case_{index}, mode| mode;"
            ),
            "finish" => format!(
                "        #[action(\"Finish from case {index}.\")]\n        let completed = |case_{index}| 7;"
            ),
            _ => format!(
                "        #[action(\"Advance in case {index}.\")]\n        |case_{index}| ();"
            ),
        })
        .collect::<Vec<_>>()
        .join("\n");
    let completes = routes.iter().any(|route| *route != "repeat");
    let transfer = if completes {
        "\n        |completed| break completed;"
    } else {
        ""
    };
    let (output, after) = if completes {
        ("let result = ", "\n    |result| return result;")
    } else {
        ("", "")
    };
    format!(
        "fn probe(mode: u8) -> u8 {{
    #[cycle(\"Exercise the generated routes.\")]
    {output}|mode| {{
        #[choice(\"Which route?\")]
{cases}
        let ({outputs}) = |mode| match mode {{
{arms}
        }};
{bodies}{transfer}
    }};{after}
}}
"
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
///
/// # Panics
///
/// Panics on a route it does not generate, for the reason [`looping`] does.
/// An unrecognized outer route also costs the nested cycle itself, which is
/// the whole point of the shape.
#[must_use]
pub fn nested(outer: &[&str], inner: &[&str]) -> String {
    assert!(
        outer
            .iter()
            .all(|route| ROUTES.contains(route) || *route == "inner"),
        "unknown outer route in {outer:?}"
    );
    assert!(
        inner.iter().all(|route| NESTED_ROUTES.contains(route)),
        "unknown inner route in {inner:?}"
    );
    let inner_selection = selection(inner, "i");
    let propagates = inner
        .iter()
        .any(|route| matches!(*route, "propagate" | "finish"));
    let inner_bodies = inner
        .iter()
        .enumerate()
        .map(|(index, route)| match (*route, propagates) {
            ("break", false) => format!(
                "        #[action(\"Complete the inner cycle from i{index}.\")]\n        let inner_value = |i{index}| ();"
            ),
            ("break", true) => format!(
                "        #[action(\"Complete only the inner cycle from i{index}.\")]\n        let inner_value = |i{index}| None;"
            ),
            ("propagate", _) => format!(
                "        #[action(\"Complete the outer cycle from i{index}.\")]\n        let inner_value = |i{index}, mode| Some(mode);"
            ),
            ("finish", _) => format!(
                "        #[action(\"Finish from i{index}.\")]\n        let inner_value = |i{index}| Some(7);"
            ),
            _ => format!(
                "        #[action(\"Advance in i{index}.\")]\n        |i{index}| ();"
            ),
        })
        .collect::<Vec<_>>()
        .join("\n");
    let inner_transfer = if inner.iter().any(|route| *route != "repeat") {
        "\n        |inner_value| break inner_value;"
    } else {
        ""
    };
    let inner_cycle = if propagates {
        format!(
            "        #[cycle(\"Exercise the generated inner routes.\")]\n        let inner_result = |mode| {{\n{inner_selection}\n{inner_bodies}{inner_transfer}\n        }};\n        #[question(\"Should the inner result complete the outer cycle?\")]\n        let (finish_outer, _repeat_outer) = |&inner_result| inner_result.is_some();\n        #[action(\"Extract the propagated inner result.\")]\n        let completed = |finish_outer, inner_result| inner_result.unwrap();"
        )
    } else {
        format!(
            "        #[cycle(\"Exercise the generated inner routes.\")]\n        |mode| {{\n{inner_selection}\n{inner_bodies}{inner_transfer}\n        }};"
        )
    };
    let outer_selection = selection(outer, "o");
    let outer_bodies = outer
        .iter()
        .enumerate()
        .map(|(index, route)| match *route {
            "break" => format!(
                "        #[action(\"Keep the mode from o{index}.\")]\n        let completed = |o{index}, mode| mode;"
            ),
            "finish" => format!(
                "        #[action(\"Finish from o{index}.\")]\n        let completed = |o{index}| 7;"
            ),
            "inner" => inner_cycle.clone(),
            _ => format!(
                "        #[action(\"Advance in o{index}.\")]\n        |o{index}| ();"
            ),
        })
        .collect::<Vec<_>>()
        .join("\n");
    let completes = outer
        .iter()
        .any(|route| matches!(*route, "break" | "finish"))
        || outer.contains(&"inner") && propagates;
    let transfer = if completes {
        "\n        |completed| break completed;"
    } else {
        ""
    };
    let (output, after) = if completes {
        ("let result = ", "\n    |result| return result;")
    } else {
        ("", "")
    };
    format!(
        "fn probe(mode: u8) -> u8 {{\n    #[cycle(\"Exercise the generated outer routes.\")]\n    {output}|mode| {{\n{outer_selection}\n{outer_bodies}{transfer}\n    }};{after}\n}}\n"
    )
}

/// The domain of the independent comparison: every flat body of two,
/// three and four routes over `repeat/break/finish`, and every three-route outer
/// body holding one inner cycle over every two-route inner body of
/// `repeat/break/propagate/finish`.
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
/// # Panics
///
/// Panics when `looping` stops emitting the choice this rewrites, which is a
/// defect in the generator rather than in what it is meant to exercise.
#[must_use]
pub fn question_shapes() -> Vec<String> {
    combinations(&ROUTES, 3)
        .iter()
        .map(|routes| {
            let mut source = looping(routes);
            let start = source
                .find("        #[choice")
                .expect("the generated cycle opens with a choice");
            let end = start
                + source[start..]
                    .find("        };")
                    .expect("the choice is closed")
                + "        };".len();
            source.replace_range(
                start..end,
                r#"        #[question("Take the first route?")]
        let (case_0, other) = |mode| mode == 0;
        #[question("Take the second route?")]
        let (case_1, case_2) = |other, mode| mode == 1;"#,
            );
            source
        })
        .collect()
}
