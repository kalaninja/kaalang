use std::sync::atomic::{AtomicUsize, Ordering};

use kaalang::kaa;

static ENTRIES: AtomicUsize = AtomicUsize::new(0);

#[kaa]
fn effect_without_wires() {
    #[kaa::action("Count the flow entry.")]
    let end = || {
        ENTRIES.fetch_add(1, Ordering::Relaxed);
    };

    |end| return end;
}

#[test]
fn an_effect_only_action_can_produce_and_return_a_unit_end_wire() {
    effect_without_wires();
    assert_eq!(ENTRIES.load(Ordering::Relaxed), 1);
}
