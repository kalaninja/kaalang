use contour::contour;

#[contour]
fn capture_one(input: u32) -> u32 {
    #[action("Increment the input.")]
    |input| -> result { input + 1 };

    #[end]
    |result| {};
}

#[contour]
fn capture_several(input: u32) -> (u32, u32, u32) {
    #[action("Build three values.")]
    |input| -> (first, second, third) { (input, input + 1, input + 2) };

    #[end]
    |third, first, second| {};
}

#[contour]
fn capture_alternative(condition: bool) -> u32 {
    #[question("Choose a result.")]
    |condition| -> (yes, no) { condition };

    #[action("Build the yes result.")]
    |yes| -> result { 11 };

    #[action("Build the no result.")]
    |no| -> result { 29 };

    #[end]
    |result| {};
}

#[contour]
fn capture_uneven(condition: bool) -> u32 {
    #[question("Choose a path depth.")]
    |condition| -> (short, long) { condition };

    #[action("Build the short result.")]
    |short| -> result { 5 };

    #[action("Prepare the long result.")]
    |long| -> prepared { 7 };

    #[action("Finish the long result.")]
    |prepared| -> result { prepared + 1 };

    #[end]
    |result| {};
}

#[contour]
fn capture_explicit_unit(input: ()) {
    #[action("Preserve the explicit unit wire.")]
    |input| -> result { input };

    #[end]
    |result| {};
}

#[contour]
fn capture_nothing() {
    #[end]
    || {};
}

#[test]
fn end_returns_one_complete_wire_value() {
    assert_eq!(capture_one(1), 2);
}

#[test]
fn end_preserves_authored_input_order() {
    assert_eq!(capture_several(1), (3, 1, 2));
}

#[test]
fn alternative_producers_feed_the_same_end_input() {
    assert_eq!(capture_alternative(true), 11);
    assert_eq!(capture_alternative(false), 29);
}

#[test]
fn paths_may_reach_end_at_different_depths() {
    assert_eq!(capture_uneven(true), 5);
    assert_eq!(capture_uneven(false), 8);
}

#[test]
fn explicit_unit_and_zero_inputs_remain_distinct_valid_forms() {
    capture_explicit_unit(());
    capture_nothing();
}
