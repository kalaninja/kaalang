//! Measures what the topology projection, the construction, and its
//! verification cost, separately from semantic analysis. Projection runs in
//! `build`; construction currently runs only when rendering. These measurements
//! also gate any future move into `build`, in the unoptimized default dev profile.
//!
//! `measure_the_construction_cost` prints the numbers and is ignored by
//! default, because a wall-clock reading is not a stable assertion. The budget
//! tests beside it assert the plan's published targets themselves, since the
//! plan makes exceeding one a no-go: a run that would be reported as a failed
//! gate has to fail the suite too.

use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use syn::ItemFn;

use crate::topology::Analyzed;

/// The costs one flow pays, and whether construction succeeded.
struct Cost {
    /// Whether the construction found a conforming arrangement. A refusal is
    /// a potential worst case: it exhausts the conflict-guided search.
    accepted: bool,
    /// Parsing, resolving, and enumerating every execution: the work `build`
    /// did before this plan.
    analysis: Duration,
    /// Projecting the topology.
    projection: Duration,
    /// Constructing an arrangement and checking it.
    construction: Duration,
}

fn cost(function: &ItemFn) -> Option<Cost> {
    let started = Instant::now();
    let mut flow = crate::parse::flow(function).ok()?;
    crate::scope::resolve(&mut flow).ok()?;
    crate::resolve::flow(&flow).ok()?;
    let (executions, _, merges) = crate::analyze::flow(&flow).ok()?;
    let execution_plan = crate::plan::flow(&flow, &executions, &merges);
    let analysis = started.elapsed();

    let started = Instant::now();
    let topology = crate::topology::project(&Analyzed {
        flow: &flow,
        executions: &executions,
        merges: &merges,
        execution_plan: &execution_plan,
    });
    let projection = started.elapsed();
    let started = Instant::now();
    let built = crate::construct::construct(&flow, &merges, &topology);
    let construction = started.elapsed();

    Some(Cost {
        accepted: built.is_ok(),
        analysis,
        projection,
        construction,
    })
}

/// Every flow of every behavior and gallery fixture.
fn corpus() -> Vec<(String, ItemFn)> {
    let tests = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../kaalang/tests")
        .canonicalize()
        .expect("the fixture tree exists");
    let mut flows = Vec::new();
    collect(&tests, &mut flows);
    flows
}

fn collect(directory: &Path, flows: &mut Vec<(String, ItemFn)>) {
    let Ok(entries) = fs::read_dir(directory) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            if path.file_name().is_some_and(|name| name == "compile_fail") {
                continue;
            }
            collect(&path, flows);
            continue;
        }
        if path.extension().is_none_or(|extension| extension != "rs") {
            continue;
        }
        let Ok(source) = fs::read_to_string(&path) else {
            continue;
        };
        let Ok(file) = syn::parse_file(&source) else {
            continue;
        };
        for item in file.items {
            if let syn::Item::Fn(function) = item
                && function
                    .attrs
                    .iter()
                    .any(|attribute| attribute.path().is_ident("kaalang"))
            {
                flows.push((function.sig.ident.to_string(), function));
            }
        }
    }
}

/// Appends one line of generated source. A `String` never fails to write.
fn writeln(body: &mut String, line: std::fmt::Arguments<'_>) {
    body.write_fmt(line).expect("writing to a String succeeds");
}

/// A flow with `loops` nested loops, each attached to its own question's
/// continuing answer and left by that question's other answer, followed by
/// `actions` pairs of straight-line blocks.
fn stress(loops: usize, actions: usize) -> ItemFn {
    let quote = '"';
    let mut body = String::new();
    let indent = |depth: usize| "    ".repeat(depth + 1);
    for depth in 0..loops {
        let pad = indent(depth * 2);
        let opening = if depth == 0 {
            "loop {".to_owned()
        } else {
            format!("|stay_{}| loop {{", depth - 1)
        };
        writeln(&mut body, format_args!("{pad}{opening}\n"));
        writeln(
            &mut body,
            format_args!("{pad}    #[question({quote}Leave level {depth}?{quote})]\n"),
        );
        writeln(
            &mut body,
            format_args!("{pad}    let (stay_{depth}, leave_{depth}) = |step| step > {depth};\n"),
        );
        writeln(&mut body, format_args!("{pad}    |leave_{depth}| break;\n"));
    }
    let deepest = loops - 1;
    let pad = indent(deepest * 2);
    writeln(
        &mut body,
        format_args!("{pad}    #[action({quote}Work at the deepest level.{quote})]\n"),
    );
    writeln(&mut body, format_args!("{pad}    |stay_{deepest}| ();\n"));
    for depth in (0..loops).rev() {
        let pad = indent(depth * 2);
        let close = if depth == 0 { "}" } else { "};" };
        writeln(&mut body, format_args!("{pad}{close}\n"));
    }
    for index in 0..actions {
        writeln(
            &mut body,
            format_args!("    #[action({quote}Step {index}.{quote})]\n"),
        );
        writeln(
            &mut body,
            format_args!("    let step_{index} = || {index}usize;\n"),
        );
        writeln(
            &mut body,
            format_args!("    #[action({quote}Use step {index}.{quote})]\n"),
        );
        writeln(&mut body, format_args!("    |step_{index}| ();\n"));
    }
    writeln(
        &mut body,
        format_args!("    #[action({quote}Finish.{quote})]\n    let end = |step| step;\n"),
    );
    let source = format!("fn stress(step: usize) -> usize {{\n{body}}}\n");
    syn::parse_str(&source).expect("the stress flow parses")
}

/// A flow of `loops` nested loops whose bodies each select `repeat, break,
/// repeat` over three cases: no arrangement of any of them conforms, so the
/// search only stops once it has exhausted the space.
fn refused(loops: usize) -> ItemFn {
    let quote = '"';
    let mut body = String::new();
    let indent = |depth: usize| "    ".repeat(depth + 1);
    for depth in 0..loops {
        let pad = indent(depth * 2);
        let opening = if depth == 0 {
            "loop {".to_owned()
        } else {
            format!("|stay_{}| loop {{", depth - 1)
        };
        writeln(&mut body, format_args!("{pad}{opening}\n"));
        writeln(
            &mut body,
            format_args!("{pad}    #[choice({quote}Which route at level {depth}?{quote})]\n"),
        );
        for case in 0..3 {
            writeln(
                &mut body,
                format_args!("{pad}    #[case({quote}Case {case} at level {depth}.{quote})]\n"),
            );
        }
        writeln(
            &mut body,
            format_args!(
                "{pad}    let (again_{depth}, leave_{depth}, stay_{depth}) = |step| match step {{\n{pad}        0 => (),\n{pad}        1 => (),\n{pad}        _ => (),\n{pad}    }};\n"
            ),
        );
        writeln(&mut body, format_args!("{pad}    |leave_{depth}| break;\n"));
        writeln(
            &mut body,
            format_args!(
                "{pad}    #[action({quote}Repeat level {depth}.{quote})]\n{pad}    |again_{depth}| ();\n"
            ),
        );
    }
    let deepest = loops - 1;
    let pad = indent(deepest * 2);
    writeln(
        &mut body,
        format_args!("{pad}    #[action({quote}Work at the deepest level.{quote})]\n"),
    );
    writeln(&mut body, format_args!("{pad}    |stay_{deepest}| ();\n"));
    for depth in (0..loops).rev() {
        let pad = indent(depth * 2);
        let close = if depth == 0 { "}" } else { "};" };
        writeln(&mut body, format_args!("{pad}{close}\n"));
    }
    writeln(
        &mut body,
        format_args!("    #[action({quote}Finish.{quote})]\n    let end = |step| step;\n"),
    );
    let source = format!("fn refused(step: usize) -> usize {{\n{body}}}\n");
    syn::parse_str(&source).expect("the refused flow parses")
}

fn median(mut samples: Vec<Duration>) -> Duration {
    samples.sort_unstable();
    samples[samples.len() / 2]
}

fn percentile(mut samples: Vec<Duration>, percent: usize) -> Duration {
    samples.sort_unstable();
    let index = samples.len() * percent / 100;
    samples[index.min(samples.len() - 1)]
}

/// One warm-up pass and twenty timed passes over the whole corpus, reporting
/// the corpus total medians and the per-flow 95th percentile.
#[test]
#[ignore = "a wall-clock measurement, not an assertion; run it with --ignored"]
fn measure_the_construction_cost() {
    let corpus = corpus();
    let mut totals = Vec::new();
    let mut analysis_totals = Vec::new();
    let mut per_flow = Vec::new();
    for run in 0..=20 {
        let mut total = Duration::ZERO;
        let mut analysis = Duration::ZERO;
        for (_, function) in &corpus {
            let Some(cost) = cost(function) else { continue };
            total += cost.construction + cost.projection;
            analysis += cost.analysis;
            if run > 0 {
                per_flow.push(cost.construction + cost.projection);
            }
        }
        if run > 0 {
            totals.push(total);
            analysis_totals.push(analysis);
        }
    }
    println!("corpus flows: {}", corpus.len());
    println!(
        "construction, corpus total median: {:?}",
        median(totals.clone())
    );
    println!(
        "analysis, corpus total median: {:?}",
        median(analysis_totals)
    );
    println!(
        "construction, per-flow 95th percentile: {:?}",
        percentile(per_flow, 95)
    );

    for (loops, actions) in [(8, 8), (8, 64), (4, 120)] {
        let function = stress(loops, actions);
        let blocks = crate::parse::flow(&function)
            .expect("the stress flow parses")
            .blocks
            .len();
        let mut samples = Vec::new();
        for run in 0..=20 {
            let Some(cost) = cost(&function) else {
                println!("stress {loops}/{actions}: rejected");
                break;
            };
            if run > 0 {
                samples.push((cost.projection, cost.construction));
            }
        }
        if !samples.is_empty() {
            println!(
                "stress {loops} loops, {blocks} blocks: projection median {:?}, construction median {:?}",
                median(samples.iter().map(|(projection, _)| *projection).collect()),
                median(
                    samples
                        .iter()
                        .map(|(_, construction)| *construction)
                        .collect()
                )
            );
        }
    }
}

/// The corpus budget of the plan: one second for a whole pass, and ten
/// milliseconds for the 95th-percentile flow.
///
/// These are the published targets themselves, not a multiple of them, because
/// the plan makes exceeding a target a no-go rather than a warning. The
/// recorded medians leave more than an order of magnitude of headroom under the
/// corpus bound and more than ten times under the per-flow one, so a loaded
/// machine does not fail the suite; a real regression does.
const CORPUS_BUDGET: Duration = Duration::from_secs(1);
const FLOW_BUDGET: Duration = Duration::from_millis(10);
/// The same for one stress flow.
const STRESS_BUDGET: Duration = Duration::from_secs(1);

/// The whole corpus is projected, constructed, and checked inside the corpus
/// budget, and the 95th-percentile flow meets the per-flow target.
#[test]
fn the_construction_stays_inside_its_budget() {
    let corpus = corpus();
    assert!(corpus.len() > 100, "the corpus should be the whole tree");
    // One warm-up pass, so the first run's page faults are not measured.
    for (_, function) in &corpus {
        let _ = cost(function);
    }
    let mut total = Duration::ZERO;
    let mut per_flow = Vec::new();
    for (name, function) in &corpus {
        let Some(cost) = cost(function) else {
            panic!("{name}: the fixture should parse")
        };
        assert!(
            cost.accepted,
            "{name}: the fixture should have an arrangement"
        );
        total += cost.construction + cost.projection;
        per_flow.push(cost.construction + cost.projection);
    }
    assert!(
        total < CORPUS_BUDGET,
        "one pass over {} flows took {total:?}, past the {CORPUS_BUDGET:?} budget",
        corpus.len()
    );
    let worst = percentile(per_flow, 95);
    assert!(
        worst < FLOW_BUDGET,
        "the 95th percentile flow took {worst:?}, past the {FLOW_BUDGET:?} budget"
    );
}

/// Every stress shape the plan names stays inside the stress budget: eight
/// nested loops, and a few hundred blocks.
///
/// Only the projection, the construction, and the check are measured. The
/// execution enumeration above them is the cost this plan did not add, and the
/// plan's bound excludes it.
#[test]
fn a_stress_flow_stays_inside_its_budget() {
    for (loops, actions) in [(8, 8), (8, 64), (4, 120)] {
        let function = stress(loops, actions);
        // One warm-up run, for the same reason as the corpus budget.
        let _ = cost(&function);
        let Some(measured) = cost(&function) else {
            panic!("the stress flow with {loops} loops should parse")
        };
        assert!(
            measured.accepted,
            "the stress flow with {loops} loops should have an arrangement"
        );
        let elapsed = measured.projection + measured.construction;
        assert!(
            elapsed < STRESS_BUDGET,
            "the stress flow with {loops} loops and {actions} steps took {elapsed:?}, past the {STRESS_BUDGET:?} budget"
        );
    }
}

/// Construction failure on nested enclosed-break flows stays inside the stress
/// budget. These cases exhaust the conflict-guided search, not every possible
/// arrangement.
#[test]
fn a_refused_flow_stays_inside_its_budget() {
    for loops in [1, 2, 3] {
        let function = refused(loops);
        let _ = cost(&function);
        let Some(measured) = cost(&function) else {
            panic!("the refused flow with {loops} loops should parse")
        };
        assert!(
            !measured.accepted,
            "the refused flow with {loops} loops should have no diagram"
        );
        let elapsed = measured.projection + measured.construction;
        assert!(
            elapsed < STRESS_BUDGET,
            "refusing the flow with {loops} loops took {elapsed:?}, past the {STRESS_BUDGET:?} budget"
        );
    }
}
