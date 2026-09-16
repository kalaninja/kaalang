//! Measures what the topology projection, the construction, and its
//! verification cost, separately from semantic analysis. `build` runs all
//! three, so every macro expansion pays them, and these measurements are the
//! gate on that in the unoptimized default dev profile.

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
    /// Projecting the topology.
    projection: Duration,
    /// Constructing an arrangement and checking it.
    construction: Duration,
}

fn cost(function: &ItemFn) -> Option<Cost> {
    let mut flow = crate::parse::flow(function).ok()?;
    crate::scope::resolve(&mut flow).ok()?;
    crate::resolve::flow(&flow).ok()?;
    let (executions, _, merges) = crate::analyze::flow(&flow).ok()?;
    let execution_plan = crate::plan::flow(&flow, &executions, &merges);

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

/// A flow with `loops` nested loops, each attached to its own question's
/// continuing answer and left by that question's other answer, followed by
/// `actions` pairs of straight-line blocks. An empty deepest tail makes the
/// enclosing tails return directly from side exits.
fn stress(loops: usize, actions: usize, empty_tail: bool) -> ItemFn {
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
    let source = format!("fn stress(step: usize) -> usize {{\n{body}}}\n");
    syn::parse_str(&source).expect("the stress flow parses")
}

/// With a distributor, a middle exit cannot pass the surrounding repeats on a
/// common case row; with ordered questions, the middle break route separates
/// two arrivals of an iteration tail it must follow.
fn branching_loops(loops: usize, actions: usize, distributor: bool) -> ItemFn {
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
    let source = format!("fn refused(mut step: usize) -> usize {{\n{body}}}\n");
    syn::parse_str(&source).expect("the refused flow parses")
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
    let source = format!("fn branching(seed: usize) -> usize {{\n{body}}}\n");
    syn::parse_str(&source).expect("the branching flow parses")
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

#[test]
fn a_serial_flow_with_a_cycle_compacts_inside_its_budget() {
    let mut model = crate::build(&stress(1, 40, false)).unwrap();
    let ranks = model.arrangement.rank.clone();
    let started = Instant::now();
    model.compact_arrangement();
    let elapsed = started.elapsed();
    assert!(
        elapsed < STRESS_BUDGET,
        "compacting a serial flow took {elapsed:?}, past the {STRESS_BUDGET:?} budget"
    );
    assert_eq!(model.arrangement.rank, ranks);
}

#[test]
fn nested_side_tails_build_and_compact_inside_their_budget() {
    let function = stress(8, 40, true);
    let measure = || {
        let started = Instant::now();
        let mut model = crate::build(&function).unwrap();
        model.compact_arrangement();
        started.elapsed()
    };
    let _ = measure();
    let elapsed = measure();
    println!("building and compacting nested side tails: {elapsed:?}");
    assert!(
        elapsed < STRESS_BUDGET,
        "building and compacting nested side tails took {elapsed:?}, past the {STRESS_BUDGET:?} budget"
    );
}

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
                stress(loops, actions, false),
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
        let function = branching_loops(loops, actions, distributor);
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
