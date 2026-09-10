use std::cell::RefCell;

use kaalang::kaalang;

thread_local! {
    static ORDER: RefCell<Vec<&'static str>> = const { RefCell::new(Vec::new()) };
}

fn record(block: &'static str) {
    ORDER.with_borrow_mut(|order| order.push(block));
}

/// A zero-output action finishes the branch before common work that captures
/// nothing from the merge. Its serial route still passes through the junction.
#[kaalang]
fn effect_then_common(flag: bool) -> u8 {
    #[question("Choose.")]
    let (yes, no) = |flag| {
        record("choose");
        flag
    };

    #[action("Yes value.")]
    let (value, local) = |yes| {
        record("yes");
        (1u8, ())
    };

    #[action("Local effect.")]
    |local| {
        record("effect");
    };

    #[action("No value.")]
    let value = |no| {
        record("no");
        2u8
    };

    #[action("Common stamp.")]
    let stamp = || {
        record("stamp");
        10u8
    };

    #[action("Finish.")]
    let end = |value, stamp| {
        record("finish");
        value + stamp
    };
}

#[test]
fn the_last_branch_effect_finishes_before_common_work() {
    for (flag, expected, trace) in [
        (
            true,
            11,
            ["choose", "yes", "effect", "stamp", "finish"].as_slice(),
        ),
        (false, 12, ["choose", "no", "stamp", "finish"].as_slice()),
    ] {
        ORDER.with_borrow_mut(Vec::clear);
        assert_eq!(effect_then_common(flag), expected);
        ORDER.with_borrow(|order| assert_eq!(order, trace));
    }
}
