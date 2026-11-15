//! What rendering costs. Nothing else bounds layout and serialization, so a
//! routing change that doubled them would show up only as a slow test suite.
//!
//! Wall-clock, in the unoptimized dev profile.

use std::time::{Duration, Instant};

use kaalang_compiler::{SourceFile, Target};
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

/// Reads the flow at `position` in `source` for no target and renders it, the
/// whole of what a caller pays for a diagram.
fn draw(source: &str, position: usize, collapse_cycles: bool) -> Result<String, String> {
    let flow = SourceFile::parse(source, Target::EMPTY)
        .and_then(|file| file.flow(position, collapse_cycles))
        .map_err(|error| error.to_string())?;
    kaalang_svg::render_flow(flow).map_err(|error| error.to_string())
}

/// The path of every flow `source` declares, read for no target, in the order
/// `SourceFile::flow` counts them.
fn flow_paths(source: &str) -> Vec<String> {
    let file = SourceFile::parse(source, Target::EMPTY).expect("the source parses");
    kaalang_compiler::flows(file.items())
        .into_iter()
        .map(|flow| flow.path)
        .collect()
}

fn check_renderer_corpus_budgets(suite: Suite, tier: &str, allowance: Duration, limit: Duration) {
    // Each diagram's flow, its position and source, and whether it is collapsed.
    let diagrams: Vec<(String, usize, String, bool)> = corpus::files(suite)
        .into_iter()
        .flat_map(|(_, source)| {
            let names = flow_paths(&source);
            // Matches what `diagrams.rs` draws: a cycle fixture gets both views.
            let views = std::iter::once(false)
                .chain(source.contains("#[kaa::cycle(").then_some(true))
                .collect::<Vec<_>>();
            names
                .into_iter()
                .enumerate()
                .flat_map(|(position, name)| {
                    views
                        .iter()
                        .map(move |&collapse_cycles| (name.clone(), position, collapse_cycles))
                })
                .map(|(name, position, collapse_cycles)| {
                    (name, position, source.clone(), collapse_cycles)
                })
                .collect::<Vec<_>>()
        })
        .collect();

    assert_pass_budget(
        &format!("renderer, {tier}"),
        "diagrams",
        &diagrams,
        allowance * u32::try_from(diagrams.len()).expect("the diagram count fits in u32"),
        |(name, _, _, collapse_cycles)| ItemBudget {
            name: format!("{name} (collapsed={collapse_cycles})"),
            limit,
            report: matches!(suite, Suite::Stress),
        },
        |(name, position, source, collapse_cycles)| {
            draw(source, *position, *collapse_cycles)
                .unwrap_or_else(|error| panic!("{name}: {error}"));
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
