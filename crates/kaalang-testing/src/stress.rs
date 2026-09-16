//! Generated flow shapes the fixture corpus does not reach: deep loop nesting,
//! a few hundred blocks, and branching that multiplies the finite executions.
//!
//! These exist to stress the decision and the geometry above it, so some of
//! them are refused on purpose. A generator returning a flow no rule rejects
//! would be the defect.

use std::fmt::Write as _;

use syn::ItemFn;

/// The flow one of these sources declares.
///
/// The generators hand back source rather than a parsed function because the
/// renderer needs the text: it slices node captions out of it by span. Callers
/// that want the function parse it here.
///
/// # Panics
///
/// Panics when the generated source does not parse, which is a defect in the
/// generator rather than in what it is meant to stress.
#[must_use]
pub fn flow(source: &str) -> ItemFn {
    let file = syn::parse_file(source).expect("the generated flow parses");
    kaalang_compiler::flows(&file.items)
        .pop()
        .expect("the generated source declares a flow")
}

/// A flow with `loops` nested loops, each attached to its own question's
/// continuing answer and left by that question's other answer, followed by
/// `actions` pairs of straight-line blocks. An empty deepest tail makes the
/// enclosing tails return directly from side exits.
///
/// # Panics
///
/// Panics when `loops` is zero: every shape here nests at least one.
#[must_use]
pub fn stress(loops: usize, actions: usize, empty_tail: bool) -> String {
    assert!(loops > 0, "a stress shape needs at least one loop");
    let quote = '"';
    let mut body = String::new();
    let indent = |depth: usize| "    ".repeat(depth + 1);
    for depth in 0..loops {
        let pad = indent(depth * 2);
        let _ = writeln!(body, "{pad}#[cycle({quote}Level {depth}.{quote})]");
        let opening = if depth == 0 {
            "|step| {".to_owned()
        } else {
            format!("|stay_{}, step| {{", depth - 1)
        };
        let _ = writeln!(body, "{pad}{opening}");
        let _ = writeln!(
            body,
            "{pad}    #[question({quote}Leave level {depth}?{quote})]"
        );
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
        let _ = writeln!(
            body,
            "{pad}    #[action({quote}Work at the deepest level.{quote})]"
        );
        let _ = writeln!(body, "{pad}    |stay_{deepest}| ();");
    }
    for depth in (0..loops).rev() {
        let pad = indent(depth * 2);
        let _ = writeln!(body, "{pad}}};");
    }
    for index in 0..actions {
        let _ = writeln!(body, "    #[action({quote}Step {index}.{quote})]");
        let _ = writeln!(body, "    let step_{index} = || {index}usize;");
        let _ = writeln!(body, "    #[action({quote}Use step {index}.{quote})]");
        let _ = writeln!(body, "    |step_{index}| ();");
    }
    let _ = writeln!(body, "    |step| return step;");
    format!("#[kaalang]\nfn stress(step: usize) -> usize {{\n{body}}}\n")
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
    let quote = '"';
    let mut body = String::new();
    let indent = |depth: usize| "    ".repeat(depth + 1);
    for depth in 0..loops {
        let pad = indent(depth * 2);
        let _ = writeln!(body, "{pad}#[cycle({quote}Level {depth}.{quote})]");
        let opening = if depth == 0 {
            "|step| {".to_owned()
        } else {
            format!("|stay_{}, step| {{", depth - 1)
        };
        let _ = writeln!(body, "{pad}{opening}");
        if distributor {
            let _ = writeln!(
                body,
                "{pad}    #[choice({quote}Which route at level {depth}?{quote})]"
            );
            for case in 0..3 {
                let _ = writeln!(
                    body,
                    "{pad}    #[case({quote}Case {case} at level {depth}.{quote})]"
                );
            }
            let _ = writeln!(
                body,
                "{pad}    let (again_{depth}, leave_{depth}, stay_{depth}) = |step| match step {{\n{pad}        0 => (),\n{pad}        1 => (),\n{pad}        _ => (),\n{pad}    }};"
            );
        } else {
            let _ = writeln!(
                body,
                "{pad}    #[question({quote}Repeat at level {depth}?{quote})]\n{pad}    let (again_{depth}, other_{depth}) = |step| step == 0;"
            );
            let _ = writeln!(
                body,
                "{pad}    #[question({quote}Leave level {depth}?{quote})]\n{pad}    let (leave_{depth}, stay_{depth}) = |other_{depth}, step| step == 1;"
            );
        }
        let _ = writeln!(body, "{pad}    |leave_{depth}| break;");
        let _ = writeln!(
            body,
            "{pad}    #[action({quote}Repeat level {depth}.{quote})]\n{pad}    |again_{depth}| ();"
        );
    }
    let deepest = loops - 1;
    let pad = indent(deepest * 2);
    let _ = writeln!(
        body,
        "{pad}    #[action({quote}Work at the deepest level.{quote})]"
    );
    let _ = writeln!(body, "{pad}    |stay_{deepest}| ();");
    for depth in (0..loops).rev() {
        let pad = indent(depth * 2);
        let _ = writeln!(body, "{pad}}};");
    }
    for action in 0..actions {
        let _ = writeln!(
            body,
            "    #[action({quote}Read step {action}.{quote})]\n    let read_{action} = |&step| *step;"
        );
        let _ = writeln!(
            body,
            "    #[action({quote}Use step {action}.{quote})]\n    |read_{action}| ();"
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
    let quote = '"';
    let mut body = String::new();
    let mut wire = "seed".to_owned();
    for stage in 0..stages {
        let _ = writeln!(body, "    #[question({quote}Take branch {stage}?{quote})]");
        let _ = writeln!(
            body,
            "    let (yes_{stage}, no_{stage}) = |{wire}| {wire} > {stage};"
        );
        let _ = writeln!(
            body,
            "    #[action({quote}Build the yes value of {stage}.{quote})]
    let step_{stage} = |yes_{stage}| {stage}usize;"
        );
        let _ = writeln!(
            body,
            "    #[action({quote}Build the no value of {stage}.{quote})]
    let step_{stage} = |no_{stage}| {stage}usize + 1;"
        );
        wire = format!("step_{stage}");
    }
    let _ = writeln!(body, "    |{wire}| return {wire};");
    format!("#[kaalang]\nfn branching(seed: usize) -> usize {{\n{body}}}\n")
}
