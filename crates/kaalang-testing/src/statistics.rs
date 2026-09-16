//! How many runs a budget samples, and the order statistic it is stated over.

use std::time::Duration;

/// How many timed passes a budget samples, past its warm-up.
pub const SAMPLES: usize = 9;

/// The nearest-rank percentile: the smallest sample at or above `percent` of
/// the set. `percentile(.., 100)` is the slowest sample.
///
/// # Panics
///
/// Panics when there are no samples.
#[must_use]
pub fn percentile(samples: &[Duration], percent: usize) -> Duration {
    assert!(!samples.is_empty(), "a budget needs at least one sample");
    let mut samples = samples.to_vec();
    samples.sort_unstable();
    let rank = (samples.len() * percent).div_ceil(100).max(1);
    samples[rank - 1]
}

/// The sample every budget is asserted on. A high percentile would report the
/// contention of running beside the rest of the suite; the median reports the
/// work, and only a regression large enough to move it is worth a failed build.
///
/// # Panics
///
/// Panics when there are no samples.
#[must_use]
pub fn median(samples: &[Duration]) -> Duration {
    percentile(samples, 50)
}

/// The distribution behind a budget, to print beside its verdict. A p100
/// several times the p50 means the machine was busy, not that something slowed.
///
/// # Panics
///
/// Panics when there are no samples.
#[must_use]
pub fn spread(samples: &[Duration]) -> String {
    // Distinct ranks at `SAMPLES`: the 5th, 7th and 9th of nine. A p90 lands on
    // the 9th too and would report the maximum twice.
    format!(
        "p50 {:?}, p75 {:?}, p100 {:?}",
        median(samples),
        percentile(samples, 75),
        percentile(samples, 100)
    )
}
