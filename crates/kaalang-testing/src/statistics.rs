//! Sampling and summary statistics shared by performance budgets.

use std::time::Duration;

/// How many timed passes a budget samples, past its warm-up.
pub const SAMPLES: usize = 9;

/// Nearest-rank percentile for `percent` in `0..=100`; zero selects the minimum.
///
/// # Panics
///
/// Panics when there are no samples. Percentages above 100 may overflow or
/// index past the samples.
#[must_use]
pub fn percentile(samples: &[Duration], percent: usize) -> Duration {
    assert!(!samples.is_empty(), "a budget needs at least one sample");
    let mut samples = samples.to_vec();
    samples.sort_unstable();
    let rank = (samples.len() * percent).div_ceil(100).max(1);
    samples[rank - 1]
}

/// Budget statistic: the median reduces sensitivity to concurrent test load.
///
/// # Panics
///
/// Panics when there are no samples.
#[must_use]
pub fn median(samples: &[Duration]) -> Duration {
    percentile(samples, 50)
}

/// Formats p50, p75, and p100 to expose outliers alongside the budget verdict.
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
