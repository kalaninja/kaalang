use kaalang::kaalang;

#[kaalang]
fn run_choice_with_shadowing(value: i32, selected: Option<i32>) -> (i32, i32) {
    #[choice("Was a value selected?")]
    #[case("A value was selected.")]
    #[case("No value was selected.")]
    let (selected_value, absent) = |&value, selected| match selected {
        Some(value) => value,
        None => (),
    };

    #[action("Produce the selected pair.")]
    let result = |selected_value, value| (value, selected_value);

    #[action("Produce the fallback pair.")]
    let result = |absent, value| (value, 0);
}

#[test]
fn choice_pattern_bindings_do_not_shadow_wires() {
    assert_eq!(run_choice_with_shadowing(7, Some(99)), (7, 99));
    assert_eq!(run_choice_with_shadowing(7, None), (7, 0));
}
