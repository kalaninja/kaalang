//! What this crate costs: analysis, topology projection, arrangement
//! construction and lowering to Rust.
//!
//! Wall-clock, in the unoptimized dev profile a macro expansion runs in.

use std::time::{Duration, Instant};

use syn::ItemFn;

use kaalang_testing::corpus;
use kaalang_testing::performance::{ItemBudget, assert_pass_budget, assert_within};
use kaalang_testing::probes::{branching, branching_loops, flow, nested_cycles};

/// The diagram-decision cost for one flow, and whether construction succeeded.
struct Cost {
    /// Whether the construction found a conforming arrangement. A refusal is
    /// a potential worst case: it exhausts the conflict-guided search.
    accepted: bool,
    /// Projection, construction and its check: the diagram decision alone. The
    /// generated-probe bounds are stated over it, since each branching stage
    /// doubles the executions analysis enumerates, which would otherwise
    /// dominate a shape built to stress the decision.
    diagram: Duration,
}

fn cost(function: &ItemFn) -> Option<Cost> {
    let analyzed = kaalang_compiler::analyze(function).ok()?;

    let started = Instant::now();
    let topology = kaalang_compiler::project(&analyzed, false);
    let built = kaalang_compiler::construct(&analyzed, &topology);
    let diagram = started.elapsed();

    if let Err(error) = &built {
        assert!(!error.to_string().contains("internal kaalang"), "{error}");
    }

    Some(Cost {
        accepted: built.is_ok(),
        diagram,
    })
}

// The corpus budgets include flows from ordinary and stress fixtures. Each flow
// is also checked against its tier's budget, so the pass catches distributed
// regressions and the flow checks catch isolated ones.

/// One model pass over the fixture corpus, against a measured median of about
/// 405 ms over 191 flows.
const MODEL_CORPUS_BUDGET: Duration = Duration::from_secs(2);
/// One ordinary fixture flow's model, against a median of about 1.9 ms.
const MODEL_FLOW_BUDGET: Duration = Duration::from_millis(25);
/// One stress-fixture flow's model. Five times the current worst median of about
/// 56 ms, rounded up.
const MODEL_STRESS_FLOW_BUDGET: Duration = Duration::from_millis(278);

/// One lowering pass over the fixture corpus, against a measured median of
/// about 565 ms. Expansion rebuilds the model internally, so this is bounded on
/// its own rather than by subtraction.
const LOWERING_CORPUS_BUDGET: Duration = Duration::from_secs(2);
/// One ordinary fixture flow's lowering, against a median of about 1.6 ms.
const LOWERING_FLOW_BUDGET: Duration = Duration::from_millis(25);
/// One stress-fixture flow's lowering. Five times the current worst median of
/// about 64 ms, rounded up.
const LOWERING_STRESS_FLOW_BUDGET: Duration = Duration::from_millis(318);

// Generated probes sit outside the fixture corpus and exercise selected stages.
/// The bound on analysis alone for a generated probe, against a measured median
/// of about 380 ms for nine branching stages.
const GENERATED_ANALYSIS_BUDGET: Duration = Duration::from_secs(2);
/// The bound on the diagram decision for a generated probe, accepted or
/// refused, against a worst measured figure of about 290 ms.
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

/// What `#[kaalang]` pays at every call site, and the only stage a user waits
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

/// Eight nested loops, a few hundred blocks, a thousand finite executions.
///
/// The loop shapes reach 259 blocks but stay in the tens of summaries;
/// `branching` is what reaches the summary counts, and eight stages is as far as
/// it goes here because `branching(n)` enumerates 2ⁿ executions.
#[test]
fn generated_accepted_probes_stay_inside_their_budget() {
    let shapes = [(8, 8), (8, 64), (4, 120)]
        .map(|(loops, actions)| {
            (
                format!("{loops} loops and {actions} steps"),
                flow(&nested_cycles(loops, actions, false)),
            )
        })
        .into_iter()
        .chain(std::iter::once((
            "eight branching stages".to_owned(),
            flow(&branching(8)),
        )));
    for (what, function) in shapes {
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

/// Analysis on its own, below the diagram decision. `branching` is what reaches
/// the execution counts: one more stage doubles them, and the placement check
/// weighs every execution against every other.
#[test]
fn a_generated_branching_probe_analyzes_inside_its_budget() {
    let probes = [("nine branching stages", flow(&branching(9)))];
    assert_pass_budget(
        "analysis",
        "probes",
        &probes,
        GENERATED_ANALYSIS_BUDGET,
        |(name, _)| ItemBudget {
            name: (*name).to_owned(),
            limit: GENERATED_ANALYSIS_BUDGET,
            report: false,
        },
        |(name, function)| {
            kaalang_compiler::analyze(function).unwrap_or_else(|error| panic!("{name}: {error}"));
        },
    );
}

/// The worst case of the whole decision: both searches run, and the deciding
/// sweep only stops once it has visited every state its space holds.
#[test]
fn generated_refused_probes_stay_inside_their_budget() {
    for (loops, actions, distributor) in [(1, 0), (4, 0), (8, 0), (8, 64), (4, 120)]
        .into_iter()
        .flat_map(|(l, a)| [(l, a, false), (l, a, true)])
    {
        let function = flow(&branching_loops(loops, actions, distributor));
        let _ = cost(&function);
        let Some(measured) = cost(&function) else {
            panic!("the generated refused probe with {loops} loops should parse")
        };
        assert!(
            !measured.accepted,
            "the generated refused probe with {loops} loops should have no diagram"
        );
        assert_within(
            &format!(
                "generated refused probe: {loops} loops, {actions} actions, choice={distributor}"
            ),
            GENERATED_DIAGRAM_DECISION_BUDGET,
            measured.diagram,
        );
    }
}
