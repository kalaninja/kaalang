//! Generated performance probes beyond fixture coverage: deep cycles, large
//! flows, and branching.

use std::fmt::Write as _;

use syn::ItemFn;

/// Parses a generated probe. Generators return source because rendering needs
/// the original text for span-based captions.
///
/// # Panics
///
/// Panics when the generated source does not parse or declares no kaalang flow.
#[must_use]
pub fn flow(source: &str) -> ItemFn {
    let file = syn::parse_file(source).expect("the generated flow parses");
    kaalang_compiler::flows(&file.items)
        .pop()
        .expect("the generated source declares a flow")
}

/// The generated shapes every budget expects to be drawn, each with what it
/// stresses and the name of the flow it declares.
#[must_use]
pub fn accepted() -> Vec<(String, String, &'static str)> {
    [(8, 8), (8, 64), (4, 120)]
        .map(|(cycles, actions)| {
            (
                format!("{cycles} cycles and {actions} steps"),
                nested_cycles(cycles, actions, false),
                "nested_cycles",
            )
        })
        .into_iter()
        .chain(std::iter::once((
            "eight branching levels".to_owned(),
            branching(8),
            "branching",
        )))
        .chain(std::iter::once((
            "staged flow with a cycle".to_owned(),
            include_str!("../../kaalang/tests/stage/behavior/digital_root.rs").to_owned(),
            "digital_root",
        )))
        .collect()
}

/// The indentation of one nesting level's cycle attribute.
fn indent(depth: usize) -> String {
    "    ".repeat(depth * 2 + 1)
}

/// One nesting level's cycle attribute and the closure gated by the enclosing
/// level's repeat wire. The level completes with its unit `_leave_{depth}`,
/// which nothing consumes. Every level is closed again by [`close_levels`].
fn open_level(depth: usize) -> String {
    let pad = indent(depth);
    let opening = if depth == 0 {
        "let _leave_0 = |step| loop {".to_owned()
    } else {
        format!("let _leave_{depth} = |stay_{}| loop {{", depth - 1)
    };
    format!("{pad}#[cycle(\"Level {depth}.\")]\n{pad}{opening}\n")
}

/// Closes every level [`open_level`] opened, innermost first, each after the
/// `continue` its staying route reaches.
fn close_levels(body: &mut String, cycles: usize) {
    for depth in (0..cycles).rev() {
        let pad = indent(depth);
        let _ = writeln!(body, "{pad}    |stay_{depth}| continue;");
        let _ = writeln!(body, "{pad}}};");
    }
}

/// Nested question-controlled cycles followed by `actions` pairs of serial blocks.
/// `empty_tail` omits the deepest action, leaving an empty repeating branch.
///
/// # Panics
///
/// Panics when `cycles` is zero: every shape here nests at least one.
#[must_use]
pub fn nested_cycles(cycles: usize, actions: usize, empty_tail: bool) -> String {
    assert!(cycles > 0, "a nested-cycle probe needs at least one cycle");
    let mut body = String::new();
    for depth in 0..cycles {
        let pad = indent(depth);
        body.push_str(&open_level(depth));
        let _ = writeln!(body, "{pad}    #[question(\"Leave level {depth}?\")]");
        let _ = writeln!(
            body,
            "{pad}    let (stay_{depth}, _leave_{depth}) = |step| step > {depth};"
        );
    }
    let deepest = cycles - 1;
    let pad = indent(deepest);
    if !empty_tail {
        let _ = writeln!(body, "{pad}    #[action(\"Work at the deepest level.\")]");
        let _ = writeln!(body, "{pad}    |stay_{deepest}| ();");
    }
    close_levels(&mut body, cycles);
    for index in 0..actions {
        let _ = writeln!(body, "    #[action(\"Step {index}.\")]");
        let _ = writeln!(body, "    let step_{index} = || {index}usize;");
        let _ = writeln!(body, "    #[action(\"Use step {index}.\")]");
        let _ = writeln!(body, "    |step_{index}| ();");
    }
    let _ = writeln!(body, "    |step| return step;");
    format!("#[kaalang]\nfn nested_cycles(step: usize) -> usize {{\n{body}}}\n")
}

/// A chain of `levels` questions, each selecting between two actions that
/// produce the same wire for the next one.
///
/// Every level doubles the possible executions. The compiler stores execution conditions for this
/// flow; explicitly listing its summaries still enumerates every
/// combination. The cycle shapes above stay in the tens however many blocks
/// they hold.
#[must_use]
pub fn branching(levels: usize) -> String {
    serial_branching(levels, false)
}

/// A serial chain whose alternatives also capture the incoming data wire.
#[must_use]
pub fn data_branching(levels: usize) -> String {
    serial_branching(levels, true)
}

/// Alternating two-way and three-way selections with branch-local work,
/// side effects, and captures of inputs and earlier merged values.
#[must_use]
pub fn branching_with_work(levels: usize) -> String {
    let mut body =
        String::from("    #[action(\"Prepare the seed.\")]\n    let prepared = |seed| seed;\n");
    let mut wire = "prepared".to_owned();
    for level in 0..levels {
        let branches = if level % 2 == 0 { 2 } else { 3 };
        if branches == 2 {
            let _ = writeln!(
                body,
                "    #[question(\"Choose work {level}.\")]\n    let (case_{level}_0, case_{level}_1) = |&config| *config & 1 == 0;"
            );
        } else {
            let _ = writeln!(
                body,
                "    #[choice(\"Choose work {level}.\")]\n    #[case(\"First.\")]\n    #[case(\"Second.\")]\n    #[case(\"Third.\")]\n    let (case_{level}_0, case_{level}_1, case_{level}_2) = |&config| match *config % 3 {{ 0 => (), 1 => (), _ => () }};"
            );
        }
        let earlier = if level > 1 {
            format!("step_{}", level / 2 - 1)
        } else {
            "prepared".to_owned()
        };
        for branch in 0..branches {
            let older = if earlier == wire {
                String::new()
            } else {
                format!(", &{earlier}")
            };
            let added = if earlier == wire {
                String::new()
            } else {
                format!(" + *{earlier}")
            };
            let offset = if branch == 0 {
                String::new()
            } else {
                format!(" + {branch}")
            };
            let _ = writeln!(
                body,
                "    #[action(\"Prepare branch {level}/{branch}.\")]\n    let data_{level}_{branch} = |case_{level}_{branch}, &{wire}, &config{older}| *{wire} + *config{added}{offset};"
            );
            let mut data = format!("data_{level}_{branch}");
            for step in 0..branch {
                let next = format!("data_{level}_{branch}_{step}");
                let _ = writeln!(
                    body,
                    "    #[call(\"Transform branch {level}/{branch}/{step}.\")]\n    let {next} = |{data}| std::convert::identity({data});"
                );
                data = next;
            }
            let _ = writeln!(
                body,
                "    #[action(\"Record branch {level}/{branch}.\")]\n    let effect_{level}_{branch} = |&{data}, &events| {{ assert_eq!(events.get(), (1usize << {level}) - 1); events.set(events.get() | (1usize << {level})); }};\n    #[action(\"Finish branch {level}/{branch}.\")]\n    let step_{level} = |{data}, effect_{level}_{branch}| {data};"
            );
        }
        wire = format!("step_{level}");
    }
    let _ = writeln!(body, "    |{wire}| return {wire};");
    format!(
        "#[kaalang]\nfn branching_with_work(seed: usize, config: usize, events: &std::cell::Cell<usize>) -> usize {{\n{body}}}\n"
    )
}

/// Branch work inside a repeating cycle with two alternative result types.
///
/// # Panics
///
/// Panics when `levels` is zero.
#[must_use]
pub fn cyclic_branching_with_work(levels: usize) -> String {
    assert!(levels > 0, "the cycle needs a data result");
    let source = branching_with_work(levels);
    let body = &source[source.find('{').expect("the probe has a body") + 1
        ..source.rfind('}').expect("the probe closes")];
    let mut body = body.replace("|seed| seed;", "|work, seed| seed;")
        .replace(&format!("|step_{}| return step_{};", levels - 1, levels - 1), &format!("#[question(\"Which result?\")] let (pick_left, pick_right) = |&step_{}, &config| *config & 1 == 0;\n#[action(\"Left result.\")] let left = |pick_left, step_{}| step_{};\n#[action(\"Right result.\")] let right = |pick_right, step_{}| (step_{},);", levels - 1, levels - 1, levels - 1, levels - 1, levels - 1));
    for level in 0..levels {
        let branches = if level % 2 == 0 { 2 } else { 3 };
        let cases = (0..branches)
            .map(|branch| format!("case_{level}_{branch}"))
            .collect::<Vec<_>>()
            .join(", ");
        let previous = if level == 0 {
            "prepared".to_owned()
        } else {
            format!("step_{}", level - 1)
        };
        body = body.replace(
            &format!("let ({cases}) = |&config|"),
            &format!("let ({cases}) = |&{previous}, &config|"),
        );
    }
    format!(
        "#[kaalang] fn cyclic_branching_with_work(seed: usize, config: usize, events: &std::cell::Cell<usize>, remaining: &std::cell::Cell<usize>) -> usize {{\n#[cycle(\"Repeat before working.\")] let (left, right) = || loop {{\n#[question(\"Repeat?\")] let (again, work) = |&remaining| remaining.get() > 0;\n#[action(\"Count the repeat.\")] let repeated = |again, &remaining| remaining.set(remaining.get() - 1);\n|repeated| continue;\n{body}\n}};\n#[action(\"Use the left result.\")] let result = |left| left;\n#[action(\"Use the right result.\")] let result = |right| right.0;\n|result| return result;\n}}"
    )
}

/// Preparation, a cycle stage, a transition stage, and a terminal stage.
///
/// # Panics
///
/// Panics when `levels` is zero.
#[must_use]
pub fn staged_branching_with_work(levels: usize) -> String {
    let source = branching_with_work(levels);
    let preparation = &source[source.find('{').expect("the probe has a body") + 1
        ..source.rfind('}').expect("the probe closes")];
    let preparation = preparation.replace(
        &format!("|step_{}| return step_{};", levels - 1, levels - 1),
        &format!(
            "#[action(\"Enter the work stage.\")] let entry = |step_{}| step_{};",
            levels - 1,
            levels - 1
        ),
    );
    let preparation = preparation
        .replace("prepared", "pre_prepared")
        .replace("case_", "pre_case_")
        .replace("data_", "pre_data_")
        .replace("effect_", "pre_effect_")
        .replace("step_", "pre_step_");
    let source = cyclic_branching_with_work(levels);
    let body = &source[source.find('{').expect("the probe has a body") + 1
        ..source.rfind('}').expect("the probe closes")];
    let body = body
        .replace("let (left, right) = ||", "let (left, right) = |reset|")
        .replace("|work, seed| seed;", "|work, entry| entry;")
        .replace(
            "let result = |left| left;",
            "let finish_left = |left| left;",
        )
        .replace(
            "let result = |right| right.0;",
            "let finish_right = |right| right.0;",
        )
        .replace("|result| return result;", "");
    format!(
        "#[kaalang] fn staged_branching_with_work(seed: usize, config: usize, events: &std::cell::Cell<usize>, remaining: &std::cell::Cell<usize>) -> usize {{\n{preparation}\n#[stage(\"Do more work.\")] let (finish_left, finish_right) = |entry| {{\n#[action(\"Reset the stage effects.\")] let reset = |entry, &events| events.set(0);\n{body}\n}};\n#[stage(\"Finish on the left.\")] let finish_right = |finish_left| {{ #[action(\"Forward the left result.\")] let finish_right = |finish_left| finish_left; }};\n#[stage(\"Finish on the right.\")] |finish_right| {{ |finish_right| return finish_right; }};\n}}"
    )
}

fn serial_branching(levels: usize, data: bool) -> String {
    let mut body = String::new();
    let mut wire = "seed".to_owned();
    for level in 0..levels {
        let _ = writeln!(body, "    #[question(\"Take branch {level}?\")]");
        let input = if data {
            format!("&{wire}")
        } else {
            wire.clone()
        };
        let value = if data {
            format!("*{wire}")
        } else {
            wire.clone()
        };
        let _ = writeln!(
            body,
            "    let (yes_{level}, no_{level}) = |{input}| {value} > {level};"
        );
        let yes = if data {
            format!("yes_{level}, {wire}")
        } else {
            format!("yes_{level}")
        };
        let no = if data {
            format!("{wire}, no_{level}")
        } else {
            format!("no_{level}")
        };
        let value = if data {
            format!("{wire} + {level}usize")
        } else {
            format!("{level}usize")
        };
        let _ = writeln!(
            body,
            "    #[action(\"Build the yes value of {level}.\")]
    let step_{level} = |{yes}| {value};"
        );
        let _ = writeln!(
            body,
            "    #[action(\"Build the no value of {level}.\")]
    let step_{level} = |{no}| {value} + 1;"
        );
        wire = format!("step_{level}");
    }
    let _ = writeln!(body, "    |{wire}| return {wire};");
    format!("#[kaalang]\nfn branching(seed: usize) -> usize {{\n{body}}}\n")
}
