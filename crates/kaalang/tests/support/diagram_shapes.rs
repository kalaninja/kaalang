//! Source generators shared by model and renderer tests. They share inputs,
//! not decision procedures or expected answers.

/// A cycle whose body selects one route per named outcome, in that order.
/// `repeat` falls through to the end of the body, `break` returns the mode, and
/// `finish` computes seven before completing the cycle.
pub(crate) fn looping(routes: &[&str]) -> String {
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

/// One cycle whose body selects `routes`, wrapped in an outer cycle whose other
/// branches take `propagate`. It uses the same names as `looping`, plus
/// `propagate` for an inner result that the enclosing cycle explicitly handles.
pub(crate) fn nested(outer: &[&str], inner: &[&str]) -> String {
    let selection = |routes: &[&str], prefix: &str| {
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
    };
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

/// The domain the plan declares for the comparison: every flat body of two,
/// three and four routes over `repeat/break/finish`, and every three-route outer
/// body holding one inner cycle over every two-route inner body of
/// `repeat/break/propagate/finish`.
pub(crate) fn declared_domain() -> Vec<String> {
    let names = ["repeat", "break", "finish"];
    let nested_names = ["repeat", "break", "propagate", "finish"];
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
    for position in 0..3 {
        for first in names {
            for second in names {
                let mut outer = vec![first, second];
                outer.insert(position, "inner");
                for left in nested_names {
                    for right in nested_names {
                        cases.push(nested(&outer, &[left, right]));
                    }
                }
            }
        }
    }
    cases
}

/// The complete generated corpus used by both model and SVG tests.
pub(crate) fn loop_shapes() -> Vec<String> {
    let names = ["repeat", "break", "finish"];
    let mut shapes = declared_domain();
    for mut code in 0..names.len().pow(5) {
        let mut routes = Vec::new();
        for _ in 0..5 {
            routes.push(names[code % names.len()]);
            code /= names.len();
        }
        shapes.push(looping(&routes));
    }
    let nested_names = ["repeat", "break", "propagate", "finish"];
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

/// The same outcomes carried by ordered question ports, rather than a shared
/// distributor exit. Both forms must respect authored branch order, while
/// their different ports and endpoints exercise different constructions.
pub(crate) fn question_shapes() -> Vec<String> {
    let mut cases = Vec::new();
    for a in ["repeat", "break", "finish"] {
        for b in ["repeat", "break", "finish"] {
            for c in ["repeat", "break", "finish"] {
                let mut source = looping(&[a, b, c]);
                let start = source.find("        #[choice").unwrap();
                let end = start + source[start..].find("        };").unwrap() + "        };".len();
                source.replace_range(
                    start..end,
                    r#"        #[question("Take the first route?")]
        let (case_0, other) = |mode| mode == 0;
        #[question("Take the second route?")]
        let (case_1, case_2) = |other, mode| mode == 1;"#,
                );
                cases.push(source);
            }
        }
    }
    cases
}
