//! What rendering costs. Nothing else bounds layout and serialization, so a
//! routing change that doubled them would show up only as a slow test suite.
//!
//! Wall-clock, in the unoptimized dev profile.

use std::time::{Duration, Instant};

use kaalang_svg::RenderOptions;
use kaalang_testing::corpus;
use kaalang_testing::performance::{ItemBudget, assert_pass_budget, assert_within};
use kaalang_testing::probes::{branching, nested_cycles};

// The corpus budget includes ordinary and stress fixtures, with both expanded
// and collapsed diagrams for every cycle fixture. Each diagram is also checked
// against its fixture tier's diagram budget.
/// One render pass over the fixture corpus, against a measured median of about
/// 1.37 s over 245 diagrams. Rendering rebuilds the model internally, so this is
/// bounded on its own rather than by subtracting the compiler's budget.
const RENDER_CORPUS_BUDGET: Duration = Duration::from_secs(4);
/// One ordinary fixture diagram, against a median of about 4.5 ms.
const RENDER_DIAGRAM_BUDGET: Duration = Duration::from_millis(60);
/// One stress-fixture diagram. Five times the current worst median of about
/// 245 ms, rounded up.
const RENDER_STRESS_DIAGRAM_BUDGET: Duration = Duration::from_millis(1225);

// Generated probes sit outside the fixture corpus.
/// One rendered generated probe, against a worst measured figure of about 4.4 s:
/// `4 loops and 120 steps` expanded, some 250 blocks. The geometry costs eight
/// to thirty-five times the decision below it at that size. Only `cargo kaalang`
/// pays this; a macro expansion never renders.
const GENERATED_RENDER_BUDGET: Duration = Duration::from_secs(12);

/// In both presentations a cycle fixture is drawn in.
#[test]
fn the_fixture_corpus_renderer_stays_inside_its_budgets() {
    let diagrams: Vec<(String, String, RenderOptions, bool)> = corpus::files()
        .into_iter()
        .flat_map(|(path, source)| {
            let stress = corpus::is_stress(&path);
            let names = kaalang_svg::flow_names(&source).expect("the fixture parses");
            // Matches what `diagrams.rs` draws: a cycle fixture gets both views.
            let views = std::iter::once(false)
                .chain(source.contains("#[cycle(").then_some(true))
                .collect::<Vec<_>>();
            names
                .into_iter()
                .flat_map(|name| {
                    views.iter().map(move |&collapse_loops| {
                        (name.clone(), RenderOptions { collapse_loops }, stress)
                    })
                })
                .map(|(name, options, stress)| (name, source.clone(), options, stress))
                .collect::<Vec<_>>()
        })
        .collect();
    assert!(!diagrams.is_empty(), "the renderer corpus is empty");

    assert_pass_budget(
        "renderer",
        "diagrams",
        &diagrams,
        RENDER_CORPUS_BUDGET,
        |(name, _, options, stress)| ItemBudget {
            name: format!("{name} (collapsed={})", options.collapse_loops),
            limit: if *stress {
                RENDER_STRESS_DIAGRAM_BUDGET
            } else {
                RENDER_DIAGRAM_BUDGET
            },
            report: *stress,
        },
        |(name, source, options, _)| {
            let drawn = kaalang_svg::render_source_with_options(source, name, *options);
            drawn.unwrap_or_else(|error| panic!("{name}: {error}"));
        },
    );
}

/// Generated probes push depth and size beyond the fixture corpus to
/// expose growth in routing and label costs.
#[test]
fn generated_probes_render_inside_their_budget() {
    let shapes = [(8, 8), (8, 64), (4, 120)]
        .map(|(loops, actions)| {
            (
                format!("{loops} loops and {actions} steps"),
                nested_cycles(loops, actions, false),
                "nested_cycles",
            )
        })
        .into_iter()
        .chain(std::iter::once((
            "eight branching stages".to_owned(),
            branching(8),
            "branching",
        )));
    for (what, source, name) in shapes {
        for collapse_loops in [false, true] {
            let options = RenderOptions { collapse_loops };
            // No warm-up: at seconds a page fault is noise, not what it removes.
            let started = Instant::now();
            let drawn = kaalang_svg::render_source_with_options(&source, name, options);
            let elapsed = started.elapsed();
            drawn.unwrap_or_else(|error| panic!("{what} (collapsed={collapse_loops}): {error}"));
            assert_within(
                &format!("generated render probe: {what}, collapsed={collapse_loops}"),
                GENERATED_RENDER_BUDGET,
                elapsed,
            );
        }
    }
}
