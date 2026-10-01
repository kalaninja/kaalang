//! What this crate costs: analysis, topology projection, arrangement
//! construction and lowering to Rust.
//!
//! Wall-clock, in the unoptimized dev profile a macro expansion runs in.

use std::time::{Duration, Instant};

use syn::ItemFn;

use kaalang_testing::corpus;
use kaalang_testing::performance::{ItemBudget, assert_pass_budget, assert_within};
use kaalang_testing::probes::{
    accepted, branching, branching_with_work, cyclic_branching_with_work, data_branching, flow,
    staged_branching_with_work,
};

/// The diagram-decision cost for one flow, and whether construction succeeded.
struct Cost {
    /// Whether the construction found a conforming arrangement. A refusal is
    /// a potential worst case: it exhausts the conflict-guided search.
    accepted: bool,
    /// Projection, construction and its check: the diagram decision alone. The
    /// generated-probe bounds are stated over it, keeping analysis separate
    /// from a shape built to stress the decision.
    diagram: Duration,
}

fn cost(function: &ItemFn) -> Option<Cost> {
    let analyzed = kaalang_compiler::analyze(function).ok()?;

    let started = Instant::now();
    let mut accepted = true;
    for part in
        std::iter::once(&analyzed).chain(analyzed.stages.iter().map(|stage| &*stage.analysis))
    {
        let topology = kaalang_compiler::project(part, false);
        let built = kaalang_compiler::construct(part, &topology);
        if let Err(error) = &built {
            assert!(!error.to_string().contains("internal kaalang"), "{error}");
        }
        accepted &= built.is_ok();
    }
    let diagram = started.elapsed();

    Some(Cost { accepted, diagram })
}

// The corpus budgets include flows from ordinary and stress fixtures. Each flow
// is also checked against its tier's budget, so the pass catches distributed
// regressions and the flow checks catch isolated ones.

/// One model pass over the fixture corpus, against a measured median of about
/// 405 ms over 191 flows.
const MODEL_CORPUS_BUDGET: Duration = Duration::from_secs(2);
/// One ordinary fixture flow's model, against a median of about 1.9 ms.
const MODEL_FLOW_BUDGET: Duration = Duration::from_millis(25);
/// One stress-fixture flow's model, against the new cycle/stage fixture's
/// median of about 310 ms, with twice that cost rounded up.
const MODEL_STRESS_FLOW_BUDGET: Duration = Duration::from_millis(640);

/// One lowering pass over the fixture corpus, against a measured median of
/// about 565 ms. Expansion rebuilds the model internally, so this is bounded on
/// its own rather than by subtraction.
const LOWERING_CORPUS_BUDGET: Duration = Duration::from_secs(2);
/// One ordinary fixture flow's lowering, against a median of about 1.6 ms.
const LOWERING_FLOW_BUDGET: Duration = Duration::from_millis(25);
/// One stress-fixture flow's lowering, with the same headroom as its model.
const LOWERING_STRESS_FLOW_BUDGET: Duration = Duration::from_millis(640);

// Generated probes sit outside the fixture corpus and exercise selected steps.
/// The bound on analysis alone for a generated probe, against a measured median
/// of about 60 ms for nine branching levels.
const GENERATED_ANALYSIS_BUDGET: Duration = Duration::from_millis(500);
/// Analysis of twelve branching levels, against a measured median of about
/// 140 ms. The 4096 possible executions remain factored during compilation.
const LARGE_GENERATED_ANALYSIS_BUDGET: Duration = Duration::from_secs(1);
/// Analysis of twenty branching levels, against a measured median of about
/// 1.75 s with branch work and older captures. That flow has over sixty million
/// executions; compilation keeps their exact conditions instead of listing them.
const SERIAL_ANALYSIS_BUDGET: Duration = Duration::from_secs(4);
/// A repeating cycle with over 120 million finite histories, against a median
/// of about 3.1 s. Preparation and a cycle stage each analyze their own histories.
const CYCLIC_ANALYSIS_BUDGET: Duration = Duration::from_secs(8);
const STAGED_ANALYSIS_BUDGET: Duration = Duration::from_secs(16);
/// The bound on the diagram decision for a generated probe, against a worst
/// measured figure of about 290 ms.
const GENERATED_DIAGRAM_DECISION_BUDGET: Duration = Duration::from_secs(3);
fn check_compiler_corpus_budgets(
    label: &str,
    corpus_budget: Duration,
    flow_budget: Duration,
    stress_flow_budget: Duration,
    run: impl Fn(&str, &ItemFn),
) {
    let flows = corpus::corpus();
    corpus::assert_corpus_shape(&flows);

    assert_pass_budget(
        label,
        "flows",
        &flows,
        corpus_budget,
        |(name, _, stress)| ItemBudget {
            name: name.clone(),
            limit: if *stress {
                stress_flow_budget
            } else {
                flow_budget
            },
            report: *stress,
        },
        |(name, function, _)| run(name, function),
    );
}

#[test]
fn the_fixture_corpus_model_stays_inside_its_budgets() {
    check_compiler_corpus_budgets(
        "model",
        MODEL_CORPUS_BUDGET,
        MODEL_FLOW_BUDGET,
        MODEL_STRESS_FLOW_BUDGET,
        |name, function| {
            let Some(cost) = cost(function) else {
                panic!("{name}: the fixture should parse")
            };
            assert!(
                cost.accepted,
                "{name}: the fixture should have an arrangement"
            );
        },
    );
}

/// What `#[kaalang]` pays at every call site, and the only step a user waits
/// on while compiling.
#[test]
fn the_fixture_corpus_lowering_stays_inside_its_budgets() {
    check_compiler_corpus_budgets(
        "lowering",
        LOWERING_CORPUS_BUDGET,
        LOWERING_FLOW_BUDGET,
        LOWERING_STRESS_FLOW_BUDGET,
        |name, function| {
            let function = function.clone();
            let lowered = kaalang_compiler::expand(function);
            lowered.unwrap_or_else(|error| panic!("{name}: {error}"));
        },
    );
}

/// Eight nested loops, a few hundred blocks, and serial questions.
///
/// The loop shapes reach 259 blocks but stay in the tens of summaries;
/// `branching` exercises factored choices. Larger chains have their own analysis
/// budget below.
#[test]
fn generated_accepted_probes_stay_inside_their_budget() {
    for (what, source, _) in accepted() {
        let function = flow(&source);
        // One warm-up run, for the same reason as the corpus budgets.
        let _ = cost(&function);
        let Some(measured) = cost(&function) else {
            panic!("the generated probe with {what} should parse")
        };
        assert!(
            measured.accepted,
            "the generated probe with {what} should have an arrangement"
        );
        assert_within(
            &format!("generated accepted probe: {what}"),
            GENERATED_DIAGRAM_DECISION_BUDGET,
            measured.diagram,
        );
    }
}

/// Analysis on its own, below the diagram decision. One more branching level
/// doubles the possible executions without requiring their enumeration.
#[test]
fn a_generated_branching_probe_analyzes_inside_its_budget() {
    for (name, source, budget) in [
        (
            "nine branching levels",
            branching(9),
            GENERATED_ANALYSIS_BUDGET,
        ),
        (
            "twelve branching levels",
            branching(12),
            LARGE_GENERATED_ANALYSIS_BUDGET,
        ),
        (
            "twenty branching levels",
            branching(20),
            SERIAL_ANALYSIS_BUDGET,
        ),
        (
            "twenty branching levels with data",
            data_branching(20),
            SERIAL_ANALYSIS_BUDGET,
        ),
        (
            "twenty branching levels with work and older captures",
            branching_with_work(20),
            SERIAL_ANALYSIS_BUDGET,
        ),
        (
            "twenty branching levels inside a repeating cycle",
            cyclic_branching_with_work(20),
            CYCLIC_ANALYSIS_BUDGET,
        ),
        (
            "twenty branching levels in preparation and a cycle stage",
            staged_branching_with_work(20),
            STAGED_ANALYSIS_BUDGET,
        ),
    ] {
        assert_pass_budget(
            &format!("analysis, {name}"),
            "probes",
            &[flow(&source)],
            budget,
            |_| ItemBudget {
                name: name.to_owned(),
                limit: budget,
                report: false,
            },
            |function| {
                kaalang_compiler::analyze(function)
                    .unwrap_or_else(|error| panic!("{name}: {error}"));
            },
        );
    }
}
