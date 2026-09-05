use std::sync::atomic::{AtomicUsize, Ordering};

use kaalang::kaalang;

static ENTRIES: AtomicUsize = AtomicUsize::new(0);

#[kaalang]
fn effect_without_wires() {
    #[action("Count the flow entry.")]
    || -> () {
        ENTRIES.fetch_add(1, Ordering::Relaxed);
    };

    #[end]
    || {};
}

#[test]
fn an_action_without_wires_runs_before_a_zero_input_end() {
    effect_without_wires();
    assert_eq!(ENTRIES.load(Ordering::Relaxed), 1);
}
