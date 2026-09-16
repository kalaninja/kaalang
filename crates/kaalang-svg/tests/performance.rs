//! What rendering costs. Nothing else bounds layout and serialization, so a
//! routing change that doubled them would show up only as a slow test suite.
//!
//! Wall-clock, in the unoptimized dev profile.

use std::time::{Duration, Instant};

use kaalang_svg::RenderOptions;
use kaalang_testing::corpus;
use kaalang_testing::statistics::{SAMPLES, median, spread};
use kaalang_testing::stress::{branching, stress};

/// One whole pass over the corpus, against a measured median of about 710 ms.
/// Rendering repeats the model build internally, so it is bounded on its own
/// rather than by subtracting the model's budget.
const CORPUS_BUDGET: Duration = Duration::from_secs(4);
/// One rendered diagram, against a median of about 4.5 ms.
const DIAGRAM_BUDGET: Duration = Duration::from_millis(60);

/// One rendered stress shape, against a worst measured figure of about 4.4 s:
/// `4 loops and 120 steps` expanded, some 250 blocks. The geometry costs eight
/// to thirty-five times the decision below it at that size. Only `cargo kaalang`
/// pays this; a macro expansion never renders.
const STRESS_BUDGET: Duration = Duration::from_secs(12);

/// In both presentations a cycle fixture is drawn in.
#[test]
fn the_renderer_stays_inside_its_budget() {
    let diagrams: Vec<(String, String, RenderOptions)> = corpus::files()
        .into_iter()
        .flat_map(|(_, source)| {
            let names = kaalang_svg::flow_names(&source).expect("the fixture parses");
            // Matches what `diagrams.rs` draws: a cycle fixture gets both views.
            let views = std::iter::once(false)
                .chain(source.contains("#[cycle(").then_some(true))
                .collect::<Vec<_>>();
            names
                .into_iter()
                .flat_map(|name| {
                    views.iter().map(move |&collapse_loops| {
                        (name.clone(), RenderOptions { collapse_loops })
                    })
                })
                .map(|(name, options)| (name, source.clone(), options))
                .collect::<Vec<_>>()
        })
        .collect();
    assert!(
        diagrams.len() > 100,
        "the renderer corpus should be the whole tree, found {}",
        diagrams.len()
    );

    let mut totals = Vec::new();
    let mut per_diagram = vec![Vec::new(); diagrams.len()];
    for run in 0..=SAMPLES {
        let mut total = Duration::ZERO;
        for (index, (name, source, options)) in diagrams.iter().enumerate() {
            let started = Instant::now();
            let drawn = kaalang_svg::render_source_with_options(source, name, *options);
            let elapsed = started.elapsed();
            drawn.unwrap_or_else(|error| panic!("{name}: {error}"));
            total += elapsed;
            if run > 0 {
                per_diagram[index].push(elapsed);
            }
        }
        if run > 0 {
            totals.push(total);
        }
    }

    let whole = median(&totals);
    println!("renderer, {} diagrams: {}", diagrams.len(), spread(&totals));
    assert!(
        whole < CORPUS_BUDGET,
        "the median render of {} diagrams took {whole:?}, past the {CORPUS_BUDGET:?} budget",
        diagrams.len()
    );
    for ((name, _, options), samples) in diagrams.iter().zip(per_diagram) {
        let typical = median(&samples);
        assert!(
            typical < DIAGRAM_BUDGET,
            "{name} (collapsed={}): the median took {typical:?}, past the {DIAGRAM_BUDGET:?} budget",
            options.collapse_loops
        );
    }
}

/// The fixtures are small hand-written examples, so the corpus above never
/// hands the geometry a large arrangement. These do, and a regression quadratic
/// in routes or labels shows up here and nowhere else.
#[test]
fn the_stress_shapes_render_inside_their_budget() {
    let shapes = [(8, 8), (8, 64), (4, 120)]
        .map(|(loops, actions)| {
            (
                format!("{loops} loops and {actions} steps"),
                stress(loops, actions, false),
                "stress",
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
            println!("stress render: {what}, collapsed={collapse_loops}: {elapsed:?}");
            assert!(
                elapsed < STRESS_BUDGET,
                "rendering {what} (collapsed={collapse_loops}) took {elapsed:?}, past the {STRESS_BUDGET:?} budget"
            );
        }
    }
}
