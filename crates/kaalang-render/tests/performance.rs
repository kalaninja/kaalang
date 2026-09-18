//! What renderer-independent structural compaction costs.
//!
//! Wall-clock, in the unoptimized dev profile used by renderer tools.

use std::time::{Duration, Instant};

use kaalang_testing::probes::{flow, nested_cycles};

/// The bound on building and compacting one generated probe, against a measured
/// median of about 740 ms before compaction moved out of the compiler crate.
const GENERATED_BUILD_AND_COMPACT_BUDGET: Duration = Duration::from_secs(3);

/// A serial flow leaves compaction almost nothing to do, so the check is that a
/// second pass finds nothing the first one left.
#[test]
fn a_generated_serial_probe_compacts_inside_its_budget() {
    let mut model =
        kaalang_compiler::build(&flow(&nested_cycles(1, 40, false))).expect("the probe builds");
    let started = Instant::now();
    kaalang_render::compact_arrangement(&mut model);
    let elapsed = started.elapsed();
    assert!(
        elapsed < GENERATED_BUILD_AND_COMPACT_BUDGET,
        "compacting a generated serial probe took {elapsed:?}, past the {GENERATED_BUILD_AND_COMPACT_BUDGET:?} budget"
    );
    let compacted = model.arrangement.clone();
    kaalang_render::compact_arrangement(&mut model);
    assert_eq!(
        model.arrangement, compacted,
        "compaction reached a fixed point"
    );
}

/// Too slow to sample the way the corpus budgets do, so it takes the fastest of
/// three runs: the one least disturbed by whatever else the machine was doing.
#[test]
fn a_generated_nested_side_tail_probe_builds_and_compacts_inside_its_budget() {
    let function = flow(&nested_cycles(8, 40, true));
    let measure = || {
        let started = Instant::now();
        let mut model = kaalang_compiler::build(&function).expect("the probe builds");
        kaalang_render::compact_arrangement(&mut model);
        started.elapsed()
    };
    let elapsed = (0..3).map(|_| measure()).min().expect("three runs");
    println!("generated nested side tail probe: {elapsed:?}");
    assert!(
        elapsed < GENERATED_BUILD_AND_COMPACT_BUDGET,
        "building and compacting a generated nested side tail probe took {elapsed:?}, past the {GENERATED_BUILD_AND_COMPACT_BUDGET:?} budget"
    );
}
