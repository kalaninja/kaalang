use kaalang::kaalang;

#[kaalang]
fn run_choice_with_shadowing(value: i32, selected: Option<i32>) -> (i32, i32) {
    #[choice("Was a value selected?")]
    #[case("A value was selected.")]
    #[case("No value was selected.")]
    |&value, selected| -> (selected_value, absent) {
        match selected {
            Some(value) => value,
            None => (),
        }
    };

    #[action("Produce the selected pair.")]
    |selected_value, value| -> result { (value, selected_value) };

    #[action("Produce the fallback pair.")]
    |absent, value| -> result { (value, 0) };

    #[end]
    |result| {};
}

#[test]
fn choice_pattern_bindings_do_not_shadow_wires() {
    assert_eq!(run_choice_with_shadowing(7, Some(99)), (7, 99));
    assert_eq!(run_choice_with_shadowing(7, None), (7, 0));
}
