use kaalang::kaalang;

#[kaalang]
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
}

#[test]
fn choice_preserves_at_patterns_and_guards() {
    assert_eq!(run_choice_with_guard(2), 2);
    assert_eq!(run_choice_with_guard(1), 0);
    assert_eq!(run_choice_with_guard(-2), 0);
}
