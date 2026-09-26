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
        .map(|(loops, actions)| {
            (
                format!("{loops} loops and {actions} steps"),
                nested_cycles(loops, actions, false),
                "nested_cycles",
            )
        })
        .into_iter()
        .chain(std::iter::once((
            "eight branching stages".to_owned(),
            branching(8),
            "branching",
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
        "let _leave_0 = |step| {".to_owned()
    } else {
        format!("let _leave_{depth} = |stay_{}| {{", depth - 1)
    };
    format!("{pad}#[cycle(\"Level {depth}.\")]\n{pad}{opening}\n")
}

/// Closes every level [`open_level`] opened, innermost first, each after the
/// `continue` its staying route reaches.
fn close_levels(body: &mut String, loops: usize) {
    for depth in (0..loops).rev() {
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
/// Panics when `loops` is zero: every shape here nests at least one.
#[must_use]
pub fn nested_cycles(loops: usize, actions: usize, empty_tail: bool) -> String {
    assert!(loops > 0, "a nested-cycle probe needs at least one loop");
    let mut body = String::new();
    for depth in 0..loops {
        let pad = indent(depth);
        body.push_str(&open_level(depth));
        let _ = writeln!(body, "{pad}    #[question(\"Leave level {depth}?\")]");
        let _ = writeln!(
            body,
            "{pad}    let (stay_{depth}, _leave_{depth}) = |step| step > {depth};"
        );
    }
    let deepest = loops - 1;
    let pad = indent(deepest);
    if !empty_tail {
        let _ = writeln!(body, "{pad}    #[action(\"Work at the deepest level.\")]");
        let _ = writeln!(body, "{pad}    |stay_{deepest}| ();");
    }
    close_levels(&mut body, loops);
    for index in 0..actions {
        let _ = writeln!(body, "    #[action(\"Step {index}.\")]");
        let _ = writeln!(body, "    let step_{index} = || {index}usize;");
        let _ = writeln!(body, "    #[action(\"Use step {index}.\")]");
        let _ = writeln!(body, "    |step_{index}| ();");
    }
    let _ = writeln!(body, "    |step| return step;");
    format!("#[kaalang]\nfn nested_cycles(step: usize) -> usize {{\n{body}}}\n")
}

/// A chain of `stages` questions, each selecting between two actions that
/// produce the same wire for the next one.
///
/// Every stage doubles the finite execution summaries, so this is the shape
/// that reaches the highest summary counts; the loop shapes above stay in the
/// tens however many blocks they hold.
#[must_use]
pub fn branching(stages: usize) -> String {
    let mut body = String::new();
    let mut wire = "seed".to_owned();
    for stage in 0..stages {
        let _ = writeln!(body, "    #[question(\"Take branch {stage}?\")]");
        let _ = writeln!(
            body,
            "    let (yes_{stage}, no_{stage}) = |{wire}| {wire} > {stage};"
        );
        let _ = writeln!(
            body,
            "    #[action(\"Build the yes value of {stage}.\")]
    let step_{stage} = |yes_{stage}| {stage}usize;"
        );
        let _ = writeln!(
            body,
            "    #[action(\"Build the no value of {stage}.\")]
    let step_{stage} = |no_{stage}| {stage}usize + 1;"
        );
        wire = format!("step_{stage}");
    }
    let _ = writeln!(body, "    |{wire}| return {wire};");
    format!("#[kaalang]\nfn branching(seed: usize) -> usize {{\n{body}}}\n")
}
