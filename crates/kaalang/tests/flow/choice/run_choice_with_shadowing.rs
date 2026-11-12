use kaalang::kaa;

#[kaa]
fn run_choice_with_shadowing(value: i32, selected: Option<i32>) -> (i32, i32) {
    #[kaa::choice("Was a value selected?")]
    #[kaa::case("A value was selected.")]
    #[kaa::case("No value was selected.")]
    let (selected_value, absent) = |&value, selected| match selected {
        Some(value) => value,
        None => (),
    };

    #[kaa::action("Produce the selected pair.")]
    let end = |selected_value, value| (value, selected_value);

    #[kaa::action("Produce the fallback pair.")]
    let end = |absent, value| (value, 0);

    |end| return end;
}

#[test]
fn choice_pattern_bindings_do_not_shadow_wires() {
    assert_eq!(run_choice_with_shadowing(7, Some(99)), (7, 99));
    assert_eq!(run_choice_with_shadowing(7, None), (7, 0));
}
