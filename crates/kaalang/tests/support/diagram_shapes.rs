//! Source generators shared by model and renderer tests. They share inputs,
//! not decision procedures or expected answers.

/// A loop whose body selects one route per named outcome, in that order.
/// `repeat` falls through to the end of the body, `break` leaves the loop, and
/// `end` finishes the flow from inside it.
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

/// One loop whose body selects `routes`, wrapped in an outer loop whose other
/// branches take `outer`. `outer` uses the same names as `looping`, plus
/// `outer` for a labelled break out of the enclosing loop.
pub(crate) fn nested(outer: &[&str], inner: &[&str]) -> String {
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

/// The domain the plan declares for the comparison: every flat body of two,
/// three and four routes over `repeat/break/end`, and every three-route outer
/// body holding one inner loop over every two-route inner body of
/// `repeat/break/outer/end`.
pub(crate) fn declared_domain() -> Vec<String> {
    let names = ["repeat", "break", "end"];
    let nested_names = ["repeat", "break", "outer", "end"];
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
    let names = ["repeat", "break", "end"];
    let mut shapes = declared_domain();
    for mut code in 0..names.len().pow(5) {
        let mut routes = Vec::new();
        for _ in 0..5 {
            routes.push(names[code % names.len()]);
            code /= names.len();
        }
        shapes.push(looping(&routes));
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

/// The same outcomes carried by ordered question ports, rather than a shared
/// distributor exit. Both forms must respect authored branch order, while
/// their different ports and endpoints exercise different constructions.
pub(crate) fn question_shapes() -> Vec<String> {
    let mut cases = Vec::new();
    for a in ["repeat", "break", "end"] {
        for b in ["repeat", "break", "end"] {
            for c in ["repeat", "break", "end"] {
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
