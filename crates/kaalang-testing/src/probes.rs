//! Generated performance probes beyond fixture coverage: deep cycles, large
//! flows, branching, and deliberately impossible topologies.

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
    let indent = |depth: usize| "    ".repeat(depth + 1);
    for depth in 0..loops {
        let pad = indent(depth * 2);
        let _ = writeln!(body, "{pad}#[cycle(\"Level {depth}.\")]");
        let opening = if depth == 0 {
            "|step| {".to_owned()
        } else {
            format!("|stay_{}, step| {{", depth - 1)
        };
        let _ = writeln!(body, "{pad}{opening}");
        let _ = writeln!(body, "{pad}    #[question(\"Leave level {depth}?\")]");
        let ignored = if empty_tail && depth + 1 == loops {
            "_"
        } else {
            ""
        };
        let _ = writeln!(
            body,
            "{pad}    let ({ignored}stay_{depth}, leave_{depth}) = |step| step > {depth};"
        );
        let _ = writeln!(body, "{pad}    |leave_{depth}| break;");
    }
    let deepest = loops - 1;
    let pad = indent(deepest * 2);
    if !empty_tail {
        let _ = writeln!(body, "{pad}    #[action(\"Work at the deepest level.\")]");
        let _ = writeln!(body, "{pad}    |stay_{deepest}| ();");
    }
    for depth in (0..loops).rev() {
        let pad = indent(depth * 2);
        let _ = writeln!(body, "{pad}}};");
    }
    for index in 0..actions {
        let _ = writeln!(body, "    #[action(\"Step {index}.\")]");
        let _ = writeln!(body, "    let step_{index} = || {index}usize;");
        let _ = writeln!(body, "    #[action(\"Use step {index}.\")]");
        let _ = writeln!(body, "    |step_{index}| ();");
    }
    let _ = writeln!(body, "    |step| return step;");
    format!("#[kaalang]\nfn nested_cycles(step: usize) -> usize {{\n{body}}}\n")
}

/// With a distributor, a middle exit cannot pass the surrounding repeats on a
/// common case row; with ordered questions, the middle break route separates
/// two arrivals of an iteration tail it must follow.
///
/// # Panics
///
/// Panics when `loops` is zero: every shape here nests at least one.
#[must_use]
pub fn branching_loops(loops: usize, actions: usize, distributor: bool) -> String {
    assert!(loops > 0, "a refused shape needs at least one loop");
    let mut body = String::new();
    let indent = |depth: usize| "    ".repeat(depth + 1);
    for depth in 0..loops {
        let pad = indent(depth * 2);
        let _ = writeln!(body, "{pad}#[cycle(\"Level {depth}.\")]");
        let opening = if depth == 0 {
            "|step| {".to_owned()
        } else {
            format!("|stay_{}, step| {{", depth - 1)
        };
        let _ = writeln!(body, "{pad}{opening}");
        if distributor {
            let _ = writeln!(
                body,
                "{pad}    #[choice(\"Which route at level {depth}?\")]"
            );
            for case in 0..3 {
                let _ = writeln!(body, "{pad}    #[case(\"Case {case} at level {depth}.\")]");
            }
            let _ = writeln!(
                body,
                "{pad}    let (again_{depth}, leave_{depth}, stay_{depth}) = |step| match step {{\n{pad}        0 => (),\n{pad}        1 => (),\n{pad}        _ => (),\n{pad}    }};"
            );
        } else {
            let _ = writeln!(
                body,
                "{pad}    #[question(\"Repeat at level {depth}?\")]\n{pad}    let (again_{depth}, other_{depth}) = |step| step == 0;"
            );
            let _ = writeln!(
                body,
                "{pad}    #[question(\"Leave level {depth}?\")]\n{pad}    let (leave_{depth}, stay_{depth}) = |other_{depth}, step| step == 1;"
            );
        }
        let _ = writeln!(body, "{pad}    |leave_{depth}| break;");
        let _ = writeln!(
            body,
            "{pad}    #[action(\"Repeat level {depth}.\")]\n{pad}    |again_{depth}| ();"
        );
    }
    let deepest = loops - 1;
    let pad = indent(deepest * 2);
    let _ = writeln!(body, "{pad}    #[action(\"Work at the deepest level.\")]");
    let _ = writeln!(body, "{pad}    |stay_{deepest}| ();");
    for depth in (0..loops).rev() {
        let pad = indent(depth * 2);
        let _ = writeln!(body, "{pad}}};");
    }
    for action in 0..actions {
        let _ = writeln!(
            body,
            "    #[action(\"Read step {action}.\")]\n    let read_{action} = |&step| *step;"
        );
        let _ = writeln!(
            body,
            "    #[action(\"Use step {action}.\")]\n    |read_{action}| ();"
        );
    }
    let _ = writeln!(body, "    |step| return step;");
    format!("#[kaalang]\nfn refused(mut step: usize) -> usize {{\n{body}}}\n")
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
