//! Shared harness for sampled performance budgets.

use std::time::{Duration, Instant};

/// How many timed passes a budget samples, past its warm-up.
const SAMPLES: usize = 9;

/// The budget and reporting policy for one item in a measured pass.
pub struct ItemBudget {
    /// Description used in output and failures.
    pub name: String,
    /// Maximum allowed median.
    pub limit: Duration,
    /// Whether to print this item's distribution on a successful run.
    pub report: bool,
}

/// Warms every item once, samples complete passes, and checks total and item
/// medians against their budgets.
///
/// # Panics
///
/// Panics when there are no items, a run panics, or a median exceeds its budget.
pub fn assert_pass_budget<T>(
    label: &str,
    unit: &str,
    items: &[T],
    pass_budget: Duration,
    describe: impl Fn(&T) -> ItemBudget,
    mut run: impl FnMut(&T),
) {
    assert!(
        !items.is_empty(),
        "a performance pass needs at least one item"
    );
    let mut totals = Vec::with_capacity(SAMPLES);
    let mut per_item = vec![Vec::with_capacity(SAMPLES); items.len()];
    for sample in 0..=SAMPLES {
        let mut total = Duration::ZERO;
        for (index, item) in items.iter().enumerate() {
            let started = Instant::now();
            run(item);
            let elapsed = started.elapsed();
            total += elapsed;
            if sample > 0 {
                per_item[index].push(elapsed);
            }
        }
        if sample > 0 {
            totals.push(total);
        }
    }

    let typical = median(&totals);
    println!("{label}, {} {unit}: {}", items.len(), spread(&totals));
    assert!(
        typical < pass_budget,
        "the median {label} pass over {} {unit} took {typical:?}, past the {pass_budget:?} budget",
        items.len()
    );
    for (item, samples) in items.iter().zip(per_item) {
        let ItemBudget {
            name,
            limit,
            report,
        } = describe(item);
        let typical = median(&samples);
        if report {
            println!("{label}, {name}: {}", spread(&samples));
        }
        assert!(
            typical < limit,
            "{name}: the median took {typical:?}, past the {limit:?} budget"
        );
    }
}

/// Reports one measured duration and checks it against its budget. Probes too
/// slow to sample the way [`assert_pass_budget`] does measure themselves and
/// report through this.
///
/// # Panics
///
/// Panics when `elapsed` reaches `budget`.
pub fn assert_within(what: &str, budget: Duration, elapsed: Duration) {
    println!("{what}: {elapsed:?}");
    assert!(
        elapsed < budget,
        "{what} took {elapsed:?}, past the {budget:?} budget"
    );
}

/// Nearest-rank percentile for `percent` in `0..=100`; zero selects the minimum.
///
/// # Panics
///
/// Panics when there are no samples. Percentages above 100 may overflow or
/// index past the samples.
fn percentile(samples: &[Duration], percent: usize) -> Duration {
    assert!(!samples.is_empty(), "a budget needs at least one sample");
    let mut samples = samples.to_vec();
    samples.sort_unstable();
    let rank = (samples.len() * percent).div_ceil(100).max(1);
    samples[rank - 1]
}

/// Budget statistic: the median reduces sensitivity to concurrent test load.
fn median(samples: &[Duration]) -> Duration {
    percentile(samples, 50)
}

/// Formats p50, p75, and p100 to expose outliers alongside the budget verdict.
fn spread(samples: &[Duration]) -> String {
    // Distinct ranks at `SAMPLES`: the 5th, 7th and 9th of nine. A p90 lands on
    // the 9th too and would report the maximum twice.
    format!(
        "p50 {:?}, p75 {:?}, p100 {:?}",
        median(samples),
        percentile(samples, 75),
        percentile(samples, 100)
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pass_budget_warms_and_samples_every_item() {
        let mut visits = 0;
        assert_pass_budget(
            "test",
            "items",
            &["first", "second"],
            Duration::from_secs(1),
            |name| ItemBudget {
                name: (*name).to_owned(),
                limit: Duration::from_secs(1),
                report: false,
            },
            |_| visits += 1,
        );

        assert_eq!(visits, (SAMPLES + 1) * 2);
    }
}
