//! Measures what the topology projection, the construction, and its
//! verification cost, separately from semantic analysis. `build` runs all
//! three, so every macro expansion pays them, and these measurements are the
//! gate on that in the unoptimized default dev profile.
//!
//! `measure_the_construction_cost` records twenty samples and asserts every
//! published target. It is ignored by default because the largest semantic
//! analysis is slow. The regular budget tests cover the corpus and smaller
//! stress shapes; both commands must pass to complete the plan.

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
    let topology = crate::topology::project(
        &Analyzed {
            flow: &flow,
            executions: &executions,
            merges: &merges,
            execution_plan: &execution_plan,
        },
        false,
    );
    let projection = started.elapsed();
    let started = Instant::now();
    let built = crate::construct::construct(&flow, &merges, &topology);
    let construction = started.elapsed();

    if let Err(error) = &built {
        assert!(!error.to_string().contains("internal kaalang"), "{error}");
    }

    Some(Cost {
        accepted: built.is_ok(),
        analysis,
        projection,
        construction,
    })
}

/// Every flow of every behavior and gallery fixture.
pub(crate) fn corpus() -> Vec<(String, ItemFn)> {
    let tests = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../kaalang/tests")
        .canonicalize()
        .expect("the fixture tree exists");
    let mut flows = Vec::new();
    collect(&tests, &mut flows);
    flows.sort_by(|a, b| a.0.cmp(&b.0));
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
        writeln(
            &mut body,
            format_args!("{pad}#[cycle({quote}Level {depth}.{quote})]\n"),
        );
        let opening = if depth == 0 {
            "|step| {".to_owned()
        } else {
            format!("|stay_{}, step| {{", depth - 1)
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
        writeln(&mut body, format_args!("{pad}}};\n"));
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
    writeln(&mut body, format_args!("    |step| return step;\n"));
    let source = format!("fn stress(step: usize) -> usize {{\n{body}}}\n");
    syn::parse_str(&source).expect("the stress flow parses")
}

/// A middle exit cannot pass the surrounding repeats on a common case row.
fn enclosed_choice(loops: usize, actions: usize) -> ItemFn {
    branching_loops(loops, actions, true)
}

/// The same separator carried by ordered question exits: the middle break
/// route separates two arrivals of an iteration tail it must follow.
fn refused(loops: usize, actions: usize) -> ItemFn {
    branching_loops(loops, actions, false)
}

fn branching_loops(loops: usize, actions: usize, distributor: bool) -> ItemFn {
    let quote = '"';
    let mut body = String::new();
    let indent = |depth: usize| "    ".repeat(depth + 1);
    for depth in 0..loops {
        let pad = indent(depth * 2);
        writeln(
            &mut body,
            format_args!("{pad}#[cycle({quote}Level {depth}.{quote})]\n"),
        );
        let opening = if depth == 0 {
            "|step| {".to_owned()
        } else {
            format!("|stay_{}, step| {{", depth - 1)
        };
        writeln(&mut body, format_args!("{pad}{opening}\n"));
        if distributor {
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
        } else {
            writeln(
                &mut body,
                format_args!(
                    "{pad}    #[question({quote}Repeat at level {depth}?{quote})]\n{pad}    let (again_{depth}, other_{depth}) = |step| step == 0;\n"
                ),
            );
            writeln(
                &mut body,
                format_args!(
                    "{pad}    #[question({quote}Leave level {depth}?{quote})]\n{pad}    let (leave_{depth}, stay_{depth}) = |other_{depth}, step| step == 1;\n"
                ),
            );
        }
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
        writeln(&mut body, format_args!("{pad}}};\n"));
    }
    for action in 0..actions {
        writeln(
            &mut body,
            format_args!(
                "    #[action({quote}Read step {action}.{quote})]\n    let read_{action} = |&step| *step;\n"
            ),
        );
        writeln(
            &mut body,
            format_args!(
                "    #[action({quote}Use step {action}.{quote})]\n    |read_{action}| ();\n"
            ),
        );
    }
    writeln(&mut body, format_args!("    |step| return step;\n"));
    let source = format!("fn refused(mut step: usize) -> usize {{\n{body}}}\n");
    syn::parse_str(&source).expect("the refused flow parses")
}

/// What one generated shape actually holds, rather than what its generator
/// arguments suggest. The plan states its stress targets in authored blocks,
/// finite execution summaries, and nested loops, so those are what a
/// measurement records.
struct Size {
    blocks: usize,
    executions: usize,
    loops: usize,
}

fn size(function: &ItemFn) -> Size {
    let mut flow = crate::parse::flow(function).expect("a generated shape parses");
    crate::scope::resolve(&mut flow).expect("a generated shape resolves");
    let (executions, _, _) = crate::analyze::flow(&flow).expect("a generated shape analyzes");
    Size {
        loops: flow
            .blocks
            .iter()
            .filter(|block| block.kind == crate::model::BlockKind::Loop)
            .count(),
        blocks: flow.blocks.len(),
        executions: executions.len(),
    }
}

/// A chain of `stages` questions, each selecting between two actions that
/// produce the same wire for the next one.
///
/// Every stage doubles the finite execution summaries, so this is the shape
/// that reaches the summary counts the plan names; the loop shapes above stay
/// in the tens however many blocks they hold.
fn branching(stages: usize) -> ItemFn {
    let quote = '"';
    let mut body = String::new();
    let mut wire = "seed".to_owned();
    for stage in 0..stages {
        writeln(
            &mut body,
            format_args!(
                "    #[question({quote}Take branch {stage}?{quote})]
"
            ),
        );
        writeln(
            &mut body,
            format_args!(
                "    let (yes_{stage}, no_{stage}) = |{wire}| {wire} > {stage};
"
            ),
        );
        writeln(
            &mut body,
            format_args!(
                "    #[action({quote}Build the yes value of {stage}.{quote})]
    let step_{stage} = |yes_{stage}| {stage}usize;
"
            ),
        );
        writeln(
            &mut body,
            format_args!(
                "    #[action({quote}Build the no value of {stage}.{quote})]
    let step_{stage} = |no_{stage}| {stage}usize + 1;
"
            ),
        );
        wire = format!("step_{stage}");
    }
    writeln(&mut body, format_args!("    |{wire}| return {wire};\n"));
    let source = format!("fn branching(seed: usize) -> usize {{\n{body}}}\n");
    syn::parse_str(&source).expect("the branching flow parses")
}

fn median(mut samples: Vec<Duration>) -> Duration {
    samples.sort_unstable();
    samples[samples.len() / 2]
}

/// The nearest-rank percentile: the smallest sample at or above `percent` of
/// the ordered set, which is the definition the plan's targets are stated in.
/// On twenty samples, p95 is the nineteenth value: the one-based rank must
/// be converted to a zero-based index.
fn percentile(mut samples: Vec<Duration>, percent: usize) -> Duration {
    samples.sort_unstable();
    let rank = (samples.len() * percent).div_ceil(100).max(1);
    samples[rank - 1]
}

/// Twenty passes over impossible choices and ordered questions.
fn measure_refusals() {
    for distributor in [false, true] {
        let label = if distributor { "choice" } else { "question" };
        for (loops, actions) in [(1, 0), (4, 0), (8, 0), (8, 64), (4, 120)] {
            let function = branching_loops(loops, actions, distributor);
            let held = size(&function);
            let mut samples = Vec::new();
            for run in 0..=20 {
                let pass = cost(&function).expect("the stress flow parses");
                assert!(!pass.accepted, "{label} {loops}/{actions}");
                if run > 0 {
                    samples.push(pass.projection + pass.construction);
                }
            }
            println!("{label} {loops}/{actions}: samples {samples:?}");
            let p95 = percentile(samples.clone(), 95);
            println!(
                "{label} {loops}/{actions}: {} blocks, {} summaries, {} loops: median {:?}, p95 {p95:?}",
                held.blocks,
                held.executions,
                held.loops,
                median(samples)
            );
            assert!(p95 < STRESS_BUDGET, "{label} {loops}/{actions}: {p95:?}");
        }
    }
}

/// The same twenty passes over the shapes that reach the summary counts the
/// plan names. Their blocks stay in the tens; what grows is the number of
/// finite executions, which the loop shapes cannot reach however many blocks
/// they hold. Enumerating those executions is what costs — it grows about
/// quadratically in their number — and that cost sits above the construction
/// this plan bounds, so it is reported apart from it.
fn measure_branching() {
    for stages in [8, 10, 12] {
        let function = branching(stages);
        // Analyzed once and then held: the enumeration is what grows here, and
        // repeating it twenty times would measure only itself.
        let started = Instant::now();
        let mut flow = crate::parse::flow(&function).expect("the branching flow parses");
        crate::scope::resolve(&mut flow).expect("the branching flow resolves");
        crate::resolve::flow(&flow).expect("the branching flow resolves");
        let (executions, _, merges) = crate::analyze::flow(&flow).expect("it analyzes");
        let execution_plan = crate::plan::flow(&flow, &executions, &merges);
        let analysis = started.elapsed();

        let mut samples = Vec::new();
        for run in 0..=20 {
            let started = Instant::now();
            let topology = crate::topology::project(
                &Analyzed {
                    flow: &flow,
                    executions: &executions,
                    merges: &merges,
                    execution_plan: &execution_plan,
                },
                false,
            );
            let built = crate::construct::construct(&flow, &merges, &topology);
            assert!(built.is_ok(), "the branching flow has an arrangement");
            if run > 0 {
                samples.push(started.elapsed());
            }
        }
        println!("branching {stages}: samples {samples:?}");
        let p95 = percentile(samples.clone(), 95);
        assert!(p95 < STRESS_BUDGET, "branching {stages}: {p95:?}");
        println!(
            "branching {stages}: {} blocks, {} summaries, analyzed once in {analysis:?}: median {:?}, p95 {:?}",
            flow.blocks.len(),
            executions.len(),
            median(samples.clone()),
            percentile(samples, 95)
        );
    }
}

/// One warm-up pass and twenty timed passes over the whole corpus, reporting
/// the corpus total medians and each flow's samples and 95th percentile.
#[test]
#[ignore = "slow debug measurements and performance gates; run it with --ignored"]
fn measure_the_construction_cost() {
    let corpus = corpus();
    let mut totals = Vec::new();
    let mut analysis_totals = Vec::new();
    let mut construction_totals = Vec::new();
    let mut per_flow = vec![Vec::new(); corpus.len()];
    for run in 0..=20 {
        let mut total = Duration::ZERO;
        let mut analysis = Duration::ZERO;
        let mut construction = Duration::ZERO;
        for (index, (_, function)) in corpus.iter().enumerate() {
            let Some(cost) = cost(function) else { continue };
            total += cost.construction + cost.projection;
            analysis += cost.analysis;
            construction += cost.construction;
            if run > 0 {
                per_flow[index].push(cost.construction + cost.projection);
            }
        }
        if run > 0 {
            totals.push(total);
            analysis_totals.push(analysis);
            construction_totals.push(construction);
        }
    }
    println!("corpus flows: {}", corpus.len());
    println!("construction, corpus totals: samples {totals:?}");
    println!("analysis, corpus totals: samples {analysis_totals:?}");
    assert!(percentile(totals.clone(), 95) < CORPUS_BUDGET);

    println!(
        "construction, corpus total: median {:?}, p95 {:?}",
        median(totals.clone()),
        percentile(totals.clone(), 95)
    );
    println!(
        "analysis, corpus total median: {:?}",
        median(analysis_totals)
    );
    for ((name, _), samples) in corpus.iter().zip(per_flow) {
        assert!(
            percentile(samples.clone(), 95) < FLOW_BUDGET,
            "{name}: {samples:?}"
        );
        println!("construction, {name}: samples {samples:?}");
        println!("construction, {name}: p95 {:?}", percentile(samples, 95));
    }
    println!(
        "construction alone, corpus total median: {:?}",
        median(construction_totals)
    );

    for (loops, actions) in [(8, 8), (8, 64), (4, 120)] {
        let function = stress(loops, actions);
        let measured = size(&function);
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
            let whole = samples
                .iter()
                .map(|(projection, construction)| *projection + *construction)
                .collect::<Vec<_>>();
            println!("stress {loops}/{actions}: samples {whole:?}");
            assert!(percentile(whole.clone(), 95) < STRESS_BUDGET);
            println!(
                "stress {loops}/{actions}: {} blocks, {} summaries, {} loops: median {:?}, p95 {:?}",
                measured.blocks,
                measured.executions,
                measured.loops,
                median(whole.clone()),
                percentile(whole, 95)
            );
        }
    }

    measure_refusals();

    measure_branching();

    let mut samples = Vec::new();
    for run in 0..=20 {
        let started = Instant::now();
        for (_, function) in &corpus {
            let _ = crate::build(function);
        }
        if run > 0 {
            samples.push(started.elapsed());
        }
    }
    println!("whole build over the corpus: samples {samples:?}");
    println!(
        "whole build over the corpus, total median: {:?}",
        median(samples)
    );
}

/// The corpus budget of the plan: one second for a whole pass, and ten
/// milliseconds for each flow's 95th percentile.
///
/// These are the published targets themselves, not a multiple of them, because
/// the plan makes exceeding a target a no-go rather than a warning. Run the
/// measured gates without concurrent heavy jobs: these are wall-clock bounds,
/// so scheduler contention can also make them fail.
const CORPUS_BUDGET: Duration = Duration::from_secs(1);
const FLOW_BUDGET: Duration = Duration::from_millis(10);
/// The published stress bound. Raising it or enabling optimization would be
/// a plan change; measurements record the hardware and default debug profile.
const STRESS_BUDGET: Duration = Duration::from_secs(1);

/// The whole corpus is projected, constructed, and checked inside the corpus
/// budget, and each flow's 95th percentile meets the per-flow target.
#[test]
fn the_construction_stays_inside_its_budget() {
    let corpus = corpus();
    assert!(corpus.len() > 100, "the corpus should be the whole tree");
    // One warm-up pass, so the first run's page faults are not measured, then
    // the twenty timed ones the plan states its targets over.
    let mut totals = Vec::new();
    let mut per_flow = vec![Vec::new(); corpus.len()];
    for run in 0..=20 {
        let mut total = Duration::ZERO;
        for (index, (name, function)) in corpus.iter().enumerate() {
            let Some(cost) = cost(function) else {
                panic!("{name}: the fixture should parse")
            };
            assert!(
                cost.accepted,
                "{name}: the fixture should have an arrangement"
            );
            total += cost.construction + cost.projection;
            if run > 0 {
                per_flow[index].push(cost.construction + cost.projection);
            }
        }
        if run > 0 {
            totals.push(total);
        }
    }
    let whole = percentile(totals, 95);
    assert!(
        whole < CORPUS_BUDGET,
        "the 95th-percentile pass over {} flows took {whole:?}, past the {CORPUS_BUDGET:?} budget",
        corpus.len()
    );
    for ((name, _), samples) in corpus.iter().zip(per_flow) {
        let p95 = percentile(samples, 95);
        assert!(
            p95 < FLOW_BUDGET,
            "{name}: the 95th percentile took {p95:?}, past the {FLOW_BUDGET:?} budget"
        );
    }
}

/// Every stress shape the plan names stays inside the stress budget: eight
/// nested loops, a few hundred blocks, and a thousand finite executions.
///
/// Only the projection, the construction, and the check are measured. The
/// execution enumeration above them is the cost this plan did not add, and the
/// plan's bound excludes it.
///
/// The sizes are recorded rather than inferred from the generator arguments.
/// The loop shapes reach 259 blocks and eight nested loops but stay in the
/// tens of summaries; `branching` is what reaches the summary counts. Eight
/// stages is what this asserts on, because enumerating the executions above the
/// construction grows about quadratically in their number and would otherwise
/// dominate the suite — and a slow assertion here is a flaky one, since every
/// budget is wall-clock. The measurement beside this records ten and twelve
/// stages, where the construction is still well inside the budget and the
/// enumeration is not this plan's cost.
#[test]
fn a_stress_flow_stays_inside_its_budget() {
    let shapes = [(8, 8), (8, 64), (4, 120)]
        .map(|(loops, actions)| {
            (
                format!("{loops} loops and {actions} steps"),
                stress(loops, actions),
            )
        })
        .into_iter()
        .chain(std::iter::once((
            "eight branching stages".to_owned(),
            branching(8),
        )));
    for (what, function) in shapes {
        eprintln!("positive budget: {what}");
        // One warm-up run, for the same reason as the corpus budget.
        let _ = cost(&function);
        let Some(measured) = cost(&function) else {
            panic!("the stress flow with {what} should parse")
        };
        assert!(
            measured.accepted,
            "the stress flow with {what} should have an arrangement"
        );
        let elapsed = measured.projection + measured.construction;
        assert!(
            elapsed < STRESS_BUDGET,
            "the stress flow with {what} took {elapsed:?}, past the {STRESS_BUDGET:?} budget"
        );
    }
}

/// Refusing a flow stays inside the stress budget at the sizes the plan names.
///
/// This is the worst case of the whole decision: both searches run, and the
/// deciding sweep only stops once it has visited every state its space holds.
#[test]
fn a_refused_flow_stays_inside_its_budget() {
    for (loops, actions, distributor) in [(1, 0), (4, 0), (8, 0), (8, 64), (4, 120)]
        .into_iter()
        .flat_map(|(l, a)| [(l, a, false), (l, a, true)])
    {
        eprintln!("refusal budget: {loops} loops, {actions} actions, choice={distributor}");
        let function = if distributor {
            enclosed_choice(loops, actions)
        } else {
            refused(loops, actions)
        };
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
            "refusing the flow with {loops} loops and {actions} steps took {elapsed:?}, past the {STRESS_BUDGET:?} budget"
        );
    }
}
