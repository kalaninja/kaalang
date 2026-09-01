use contour::contour;

#[contour]
fn run_choice(value: i32, branch_action_count: &mut usize) -> &'static str {
    #[choice("What is the sign of the value?")]
    #[case("The value is negative.")]
    #[case("The value is zero.")]
    #[case("The value is positive.")]
    |value| -> (negative, zero, positive) {
        match value {
            ..0 => (),
            0 => (),
            _ => (),
        }
    };

    #[action("Produce the negative result.")]
    |negative, branch_action_count| -> result {
        *branch_action_count += 1;
        "negative"
    };

    #[action("Produce the zero result.")]
    |zero, branch_action_count| -> result {
        *branch_action_count += 1;
        "zero"
    };

    #[action("Produce the positive result.")]
    |positive, branch_action_count| -> result {
        *branch_action_count += 1;
        "positive"
    };

    #[end]
    |result| {};
}

#[allow(unreachable_code)]
#[contour]
fn run_choice_skeleton(value: i32) -> &'static str {
    #[choice("What is the sign of the value?")]
    #[case("The value is negative.")]
    #[case("The value is nonnegative.")]
    |value| -> (negative, nonnegative) { todo!() };

    #[action("Produce the negative result.")]
    |negative| -> result { todo!() };

    #[action("Produce the nonnegative result.")]
    |nonnegative| -> result { todo!() };

    #[end]
    |result| {};
}

#[contour]
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

#[contour]
fn run_choice_with_guard(value: i32) -> i32 {
    #[choice("Is the value a positive even number?")]
    #[case("The value is positive and even.")]
    #[case("The value is not positive and even.")]
    |value| -> (positive_even, other) {
        match value {
            matched @ 1.. if matched % 2 == 0 => matched,
            _ => (),
        }
    };

    #[action("Produce the positive-even result.")]
    |positive_even| -> result { positive_even };

    #[action("Produce the other result.")]
    |other| -> result { 0 };

    #[end]
    |result| {};
}

#[test]
fn choice_executes_each_branch() {
    let mut branch_action_count = 0;

    assert_eq!(run_choice(-1, &mut branch_action_count), "negative");
    assert_eq!(branch_action_count, 1);
    assert_eq!(run_choice(0, &mut branch_action_count), "zero");
    assert_eq!(branch_action_count, 2);
    assert_eq!(run_choice(1, &mut branch_action_count), "positive");
    assert_eq!(branch_action_count, 3);
    assert_eq!(run_choice(2, &mut branch_action_count), "positive");
    assert_eq!(branch_action_count, 4);
}

#[test]
fn choice_pattern_bindings_do_not_shadow_wires() {
    assert_eq!(run_choice_with_shadowing(7, Some(99)), (7, 99));
    assert_eq!(run_choice_with_shadowing(7, None), (7, 0));
}

#[test]
fn choice_preserves_at_patterns_and_guards() {
    assert_eq!(run_choice_with_guard(2), 2);
    assert_eq!(run_choice_with_guard(1), 0);
    assert_eq!(run_choice_with_guard(-2), 0);
}

#[test]
#[should_panic(expected = "not yet implemented")]
fn choice_skeleton_panics_only_when_executed() {
    run_choice_skeleton(0);
}
