use std::sync::atomic::{AtomicUsize, Ordering};

use kaalang::kaalang;

static VISITS: AtomicUsize = AtomicUsize::new(0);

/// The bare spelling of an effect: no inputs, no outputs, at flow entry.
#[kaalang]
fn entry_effect_without_wires() -> usize {
    #[action("Count the flow entry.")]
    || {
        VISITS.fetch_add(1, Ordering::Relaxed);
    };

    #[action("Report the count.")]
    let result = || VISITS.load(Ordering::Relaxed);
}

#[test]
fn an_effect_with_no_wires_at_all_runs_before_the_result() {
    assert_eq!(entry_effect_without_wires(), 1);
}
