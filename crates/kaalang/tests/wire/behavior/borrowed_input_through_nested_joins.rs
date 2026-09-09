use kaalang::kaalang;

/// A flow input owns the text outside both branch scopes. Its reference can
/// cross the inner and outer joins while the input remains alive.
#[kaalang]
fn borrowed_input_through_nested_joins(outer: bool, inner: bool, text: String) -> usize {
    #[question("Use the nested branch?")]
    |outer| -> (nested, fallback) { outer };

    #[question("Borrow the flow input?")]
    |nested, inner| -> (borrow, skip) { inner };

    #[action("Borrow the flow input.")]
    |borrow, &text| -> partial { text.as_str() };

    #[action("Use the inner fallback.")]
    |skip| -> partial { "inner" };

    #[action("Carry the inner view onward.")]
    |partial| -> view { partial };

    #[action("Use the outer fallback.")]
    |fallback| -> view { "fallback" };

    #[action("Measure the final view.")]
    |view| -> result { view.len() };
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
