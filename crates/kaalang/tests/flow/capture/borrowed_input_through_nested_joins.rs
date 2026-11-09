use kaalang::kaa;

/// A flow input owns the text outside both branch scopes. Its reference can
/// cross the inner and outer joins while the input remains alive.
#[kaa]
fn borrowed_input_through_nested_joins(outer: bool, inner: bool, text: String) -> usize {
    #[kaa::question("Use the nested branch?")]
    let (nested, fallback) = |outer| outer;

    #[kaa::question("Borrow the flow input?")]
    let (borrow, skip) = |nested, inner| inner;

    #[kaa::action("Borrow the flow input.")]
    let partial = |borrow, &text| text.as_str();

    #[kaa::action("Use the inner fallback.")]
    let partial = |skip| "inner";

    #[kaa::action("Carry the inner view onward.")]
    let view = |partial| partial;

    #[kaa::action("Use the outer fallback.")]
    let view = |fallback| "fallback";

    #[kaa::action("Measure the final view.")]
    let end = |view| view.len();

    |end| return end;
}

#[test]
fn a_reference_to_the_flow_input_crosses_both_joins() {
    for (outer, inner, expected) in [
        (true, true, 3),
        (true, false, 5),
        (false, true, 8),
        (false, false, 8),
    ] {
        assert_eq!(
            borrowed_input_through_nested_joins(outer, inner, String::from("abc")),
            expected
        );
    }
}
