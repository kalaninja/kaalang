use std::sync::atomic::{AtomicUsize, Ordering};

use kaalang::kaa;

static ARRIVALS: AtomicUsize = AtomicUsize::new(0);

fn record() {
    ARRIVALS.fetch_add(1, Ordering::Relaxed);
}

#[kaa]
fn call_a_bare_application() -> usize {
    #[kaa::call]
    record();

    #[kaa::action("Report the recorded arrivals.")]
    let end = || ARRIVALS.load(Ordering::Relaxed);

    |end| return end;
}

#[test]
fn a_call_with_neither_inputs_nor_outputs_needs_no_braces() {
    assert_eq!(call_a_bare_application(), 1);
}
