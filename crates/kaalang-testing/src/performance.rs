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

    let [typical, upper, maximum] = summary(totals);
    println!(
        "{label}, {} {unit}: p50 {typical:?}, p75 {upper:?}, p100 {maximum:?}",
        items.len()
    );
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
        let [typical, upper, maximum] = summary(samples);
        if report {
            println!("{label}, {name}: p50 {typical:?}, p75 {upper:?}, p100 {maximum:?}");
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

/// Nearest-rank p50, p75, and p100 for one complete budget sample.
/// The median reduces sensitivity to concurrent test load; the rest expose outliers.
fn summary(mut samples: Vec<Duration>) -> [Duration; 3] {
    assert_eq!(samples.len(), SAMPLES, "a budget pass samples every item");
    samples.sort_unstable();
    [
        samples[(SAMPLES - 1) / 2],
        samples[(SAMPLES * 3).div_ceil(4) - 1],
        samples[SAMPLES - 1],
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn summary_uses_nearest_ranks() {
        let samples = [9, 1, 8, 2, 7, 3, 6, 4, 5]
            .map(Duration::from_millis)
            .to_vec();
        assert_eq!(summary(samples), [5, 7, 9].map(Duration::from_millis));
    }

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
