//! What rendering costs. Nothing else bounds layout and serialization, so a
//! routing change that doubled them would show up only as a slow test suite.
//!
//! Wall-clock, in the unoptimized dev profile.

use std::time::{Duration, Instant};

use kaalang_compiler::SourceFile;
use kaalang_testing::{
    corpus::{self, Suite},
    performance::{ItemBudget, assert_pass_budget, assert_within},
    probes::accepted,
};

// Ordinary and stress fixtures have separate corpus passes, with both expanded
// and collapsed diagrams for every cycle fixture. Each diagram is also checked
// against its fixture tier's diagram budget.
/// Per-diagram allowance for 353 ordinary diagrams: serial p50 4.72 s on a core
/// with 1.8 GHz base frequency and Turbo Boost disabled, with about 50% headroom.
/// A diagram is read from its source and rendered, so its budget stands on its own.
const RENDER_CORPUS_DIAGRAM_BUDGET: Duration = Duration::from_millis(20);
/// Per-diagram allowance for seven stress diagrams: serial p50 3.63 s on the
/// same core, with about 50% headroom.
const RENDER_CORPUS_STRESS_DIAGRAM_BUDGET: Duration = Duration::from_millis(800);
/// One ordinary fixture diagram: collapsed `kmp_search` measured a median of
/// 62 ms on the same core.
const RENDER_DIAGRAM_BUDGET: Duration = Duration::from_millis(100);
/// One stress-fixture diagram, against a worst measured median of about 1.00 s
/// on the same core.
const RENDER_STRESS_DIAGRAM_BUDGET: Duration = Duration::from_millis(1225);

// Generated probes sit outside the fixture corpus.
/// One rendered generated probe, against a worst measured figure of about 4.4 s:
/// `4 cycles and 120 steps` expanded, some 250 blocks. The geometry costs eight
/// to thirty-five times the decision below it at that size. Only `cargo kaalang`
/// pays this; a macro expansion never renders.
const GENERATED_RENDER_BUDGET: Duration = Duration::from_secs(12);

/// Reads the flow at `position` for the fixture target and renders it.
fn draw(source: &str, position: usize, collapse_cycles: bool) -> Result<String, String> {
    let flow = SourceFile::parse(source, &corpus::TARGET)
        .and_then(|file| file.flow(position, collapse_cycles))
        .map_err(|error| error.to_string())?;
    kaalang_svg::render_flow(flow)
        .map(|(svg, _)| svg)
        .map_err(|error| error.to_string())
}

/// The path of every flow `source` declares for the fixture target, in the order
/// `SourceFile::flow` counts them.
fn flow_paths(source: &str) -> Vec<String> {
    let file = SourceFile::parse(source, &corpus::TARGET).expect("the source parses");
    kaalang_compiler::flows(file.items())
        .into_iter()
        .map(|flow| flow.path)
        .collect()
}

fn check_renderer_corpus_budgets(suite: Suite, tier: &str, allowance: Duration, limit: Duration) {
    let diagrams = corpus::diagrams(suite);

    assert_pass_budget(
        &format!("renderer, {tier}"),
        "diagrams",
        &diagrams,
        allowance * u32::try_from(diagrams.len()).expect("the diagram count fits in u32"),
        |diagram| ItemBudget {
            name: diagram.to_string(),
            limit,
            report: matches!(suite, Suite::Stress),
        },
        |diagram| {
            draw(&diagram.source, diagram.position, diagram.collapse_cycles)
                .unwrap_or_else(|error| panic!("{diagram}: {error}"));
        },
    );
}

#[test]
fn the_ordinary_fixture_corpus_renderer_stays_inside_its_budgets() {
    check_renderer_corpus_budgets(
        Suite::Ordinary,
        "ordinary",
        RENDER_CORPUS_DIAGRAM_BUDGET,
        RENDER_DIAGRAM_BUDGET,
    );
}

#[test]
fn the_stress_fixture_corpus_renderer_stays_inside_its_budgets() {
    check_renderer_corpus_budgets(
        Suite::Stress,
        "stress",
        RENDER_CORPUS_STRESS_DIAGRAM_BUDGET,
        RENDER_STRESS_DIAGRAM_BUDGET,
    );
}

/// Generated probes push depth and size beyond the fixture corpus to
/// expose growth in routing and label costs.
#[test]
fn generated_probes_render_inside_their_budget() {
    for (what, source, name) in accepted() {
        for collapse_cycles in [false, true] {
            let position = flow_paths(&source)
                .iter()
                .position(|listed| listed == name)
                .expect("the probe declares its flow");
            // No warm-up: at seconds a page fault is noise, not what it removes.
            let started = Instant::now();
            let drawn = draw(&source, position, collapse_cycles);
            let elapsed = started.elapsed();
            drawn.unwrap_or_else(|error| panic!("{what} (collapsed={collapse_cycles}): {error}"));
            assert_within(
                &format!("generated render probe: {what}, collapsed={collapse_cycles}"),
                GENERATED_RENDER_BUDGET,
                elapsed,
            );
        }
    }
}
