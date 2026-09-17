//! What this crate costs: analysis, topology projection, arrangement
//! construction, compaction, and lowering to Rust.
//!
//! Wall-clock, in the unoptimized dev profile a macro expansion runs in.

use std::time::{Duration, Instant};

use syn::ItemFn;

use kaalang_testing::corpus;
use kaalang_testing::statistics::{SAMPLES, median, spread};
use kaalang_testing::stress::{branching, branching_loops, flow, stress};

/// The costs one flow pays before any Rust is emitted, and whether
/// construction succeeded.
struct Cost {
    /// Whether the construction found a conforming arrangement. A refusal is
    /// a potential worst case: it exhausts the conflict-guided search.
    accepted: bool,
    /// Parsing, resolving, enumerating executions, and planning the lowering.
    analysis: Duration,
    /// Projecting the topology.
    projection: Duration,
    /// Constructing an arrangement and checking it.
    construction: Duration,
}

impl Cost {
    /// The diagram decision alone. The stress bounds are stated over this:
    /// enumeration grows about quadratically in the branching stages and would
    /// otherwise dominate a shape built to stress the decision.
    fn diagram(&self) -> Duration {
        self.projection + self.construction
    }

    /// Everything a macro expansion pays before code generation.
    fn whole(&self) -> Duration {
        self.analysis + self.diagram()
    }
}

fn cost(function: &ItemFn) -> Option<Cost> {
    let started = Instant::now();
    let analyzed = kaalang_compiler::analyze(function).ok()?;
    let analysis = started.elapsed();

    let started = Instant::now();
    let topology = kaalang_compiler::project(&analyzed, false);
    let projection = started.elapsed();

    let started = Instant::now();
    let built = kaalang_compiler::construct(&analyzed, &topology);
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

/// One whole pass over the corpus, from source to checked arrangement, against
/// a measured median of about 290 ms over 190 flows.
const CORPUS_BUDGET: Duration = Duration::from_secs(2);
/// One flow's own share of that pass, against a median of about 1.9 ms. The
/// aggregate above cannot see a single flow whose cost explodes; this can.
const FLOW_BUDGET: Duration = Duration::from_millis(25);

/// One whole pass of `expand` over the corpus, against a measured median of
/// about 430 ms. It repeats the model build internally, so it is bounded on its
/// own rather than by subtraction.
const LOWERING_CORPUS_BUDGET: Duration = Duration::from_secs(2);
/// One lowered flow, against a median of about 1.6 ms.
const LOWERING_FLOW_BUDGET: Duration = Duration::from_millis(25);

/// The bound on the diagram decision for a generated shape, accepted or
/// refused, against a worst measured figure of about 290 ms.
const STRESS_BUDGET: Duration = Duration::from_secs(3);
/// The bound on building and compacting one, which unlike [`STRESS_BUDGET`]
/// includes the analysis above the decision. Against about 740 ms.
const BUILD_AND_COMPACT_BUDGET: Duration = Duration::from_secs(3);

/// Times every corpus flow `SAMPLES` times over, then checks the median whole
/// pass against `corpus_budget` and each flow's own median against
/// `flow_budget`. The aggregate cannot see a single flow whose cost explodes.
fn corpus_budget(
    label: &str,
    corpus_budget: Duration,
    flow_budget: Duration,
    measure: impl Fn(&str, &ItemFn) -> Duration,
) {
    let flows = corpus::corpus();
    corpus::assert_whole_tree(&flows);

    let mut totals = Vec::new();
    let mut per_flow = vec![Vec::new(); flows.len()];
    for run in 0..=SAMPLES {
        let mut total = Duration::ZERO;
        for (index, (name, function)) in flows.iter().enumerate() {
            let elapsed = measure(name, function);
            total += elapsed;
            if run > 0 {
                per_flow[index].push(elapsed);
            }
        }
        if run > 0 {
            totals.push(total);
        }
    }

    let whole = median(&totals);
    println!("{label}, {} flows: {}", flows.len(), spread(&totals));
    assert!(
        whole < corpus_budget,
        "the median {label} pass over {} flows took {whole:?}, past the {corpus_budget:?} budget",
        flows.len()
    );
    for ((name, _), samples) in flows.iter().zip(per_flow) {
        let typical = median(&samples);
        assert!(
            typical < flow_budget,
            "{name}: the median took {typical:?}, past the {flow_budget:?} budget"
        );
    }
}

#[test]
fn the_model_stays_inside_its_budget() {
    corpus_budget("model", CORPUS_BUDGET, FLOW_BUDGET, |name, function| {
        let Some(cost) = cost(function) else {
            panic!("{name}: the fixture should parse")
        };
        assert!(
            cost.accepted,
            "{name}: the fixture should have an arrangement"
        );
        cost.whole()
    });
}

/// What `#[kaalang]` pays at every call site, and the only stage a user waits
/// on while compiling.
#[test]
fn the_lowering_stays_inside_its_budget() {
    corpus_budget(
        "lowering",
        LOWERING_CORPUS_BUDGET,
        LOWERING_FLOW_BUDGET,
        |name, function| {
            let function = function.clone();
            let started = Instant::now();
            let lowered = kaalang_compiler::expand(function);
            let elapsed = started.elapsed();
            lowered.unwrap_or_else(|error| panic!("{name}: {error}"));
            elapsed
        },
    );
}

/// A serial flow leaves compaction almost nothing to do, so the check is that a
/// second pass finds nothing the first one left.
#[test]
fn a_serial_flow_with_a_cycle_compacts_inside_its_budget() {
    let mut model =
        kaalang_compiler::build(&flow(&stress(1, 40, false))).expect("the stress flow builds");
    let started = Instant::now();
    model.compact_arrangement();
    let elapsed = started.elapsed();
    assert!(
        elapsed < BUILD_AND_COMPACT_BUDGET,
        "compacting a serial flow took {elapsed:?}, past the {BUILD_AND_COMPACT_BUDGET:?} budget"
    );
    let ranks = model.arrangement.rank.clone();
    model.compact_arrangement();
    assert_eq!(
        model.arrangement.rank, ranks,
        "compaction reached a fixed point"
    );
}

/// Too slow to sample the way the corpus budgets do, so it takes the fastest of
/// three runs: the one least disturbed by whatever else the machine was doing.
#[test]
fn nested_side_tails_build_and_compact_inside_their_budget() {
    let function = flow(&stress(8, 40, true));
    let measure = || {
        let started = Instant::now();
        let mut model = kaalang_compiler::build(&function).expect("the stress flow builds");
        model.compact_arrangement();
        started.elapsed()
    };
    let elapsed = (0..3).map(|_| measure()).min().expect("three runs");
    println!("building and compacting nested side tails: {elapsed:?}");
    assert!(
        elapsed < BUILD_AND_COMPACT_BUDGET,
        "building and compacting nested side tails took {elapsed:?}, past the {BUILD_AND_COMPACT_BUDGET:?} budget"
    );
}

/// Eight nested loops, a few hundred blocks, a thousand finite executions.
///
/// The loop shapes reach 259 blocks but stay in the tens of summaries;
/// `branching` is what reaches the summary counts, and eight stages is as far as
/// it goes here because enumeration grows about quadratically in them.
#[test]
fn a_stress_flow_stays_inside_its_budget() {
    let shapes = [(8, 8), (8, 64), (4, 120)]
        .map(|(loops, actions)| {
            (
                format!("{loops} loops and {actions} steps"),
                flow(&stress(loops, actions, false)),
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
            panic!("the stress flow with {what} should parse")
        };
        assert!(
            measured.accepted,
            "the stress flow with {what} should have an arrangement"
        );
        let elapsed = measured.diagram();
        println!("positive budget: {what}: {elapsed:?}");
        assert!(
            elapsed < STRESS_BUDGET,
            "the stress flow with {what} took {elapsed:?}, past the {STRESS_BUDGET:?} budget"
        );
    }
}

/// The worst case of the whole decision: both searches run, and the deciding
/// sweep only stops once it has visited every state its space holds.
#[test]
fn a_refused_flow_stays_inside_its_budget() {
    for (loops, actions, distributor) in [(1, 0), (4, 0), (8, 0), (8, 64), (4, 120)]
        .into_iter()
        .flat_map(|(l, a)| [(l, a, false), (l, a, true)])
    {
        let function = flow(&branching_loops(loops, actions, distributor));
        let _ = cost(&function);
        let Some(measured) = cost(&function) else {
            panic!("the refused flow with {loops} loops should parse")
        };
        assert!(
            !measured.accepted,
            "the refused flow with {loops} loops should have no diagram"
        );
        let elapsed = measured.diagram();
        println!(
            "refusal budget: {loops} loops, {actions} actions, choice={distributor}: {elapsed:?}"
        );
        assert!(
            elapsed < STRESS_BUDGET,
            "refusing the flow with {loops} loops and {actions} steps took {elapsed:?}, past the {STRESS_BUDGET:?} budget"
        );
    }
}
