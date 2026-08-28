use contour::contour;

#[contour]
fn run_choice(value: i32) -> &'static str {
    #[choice("What is the sign of the value?")]
    #[case("The value is negative.")]
    #[case("The value is zero.")]
    #[case("The value is positive.")]
    |value| -> (negative, zero, positive) {
        if value < 0 {
            negative
        } else if value == 0 {
            zero
        } else {
            positive
        }
    };

    #[action("Return the negative result.")]
    |negative| -> negative_result { "negative" };

    #[action("Return the zero result.")]
    |zero| -> zero_result { "zero" };

    #[action("Return the positive result.")]
    |positive| -> positive_result { "positive" };
}

#[contour]
fn run_choice_skeleton(value: i32) -> &'static str {
    #[choice("What is the sign of the value?")]
    #[case("The value is negative.")]
    #[case("The value is nonnegative.")]
    |value| -> (negative, nonnegative) { todo!() };

    #[action("Return the negative result.")]
    |negative| -> negative_result { todo!() };

    #[action("Return the nonnegative result.")]
    |nonnegative| -> nonnegative_result { todo!() };
}

#[test]
fn choice_block_routes_each_case() {
    assert_eq!(run_choice(-1), "negative");
    assert_eq!(run_choice(0), "zero");
    assert_eq!(run_choice(1), "positive");
}

#[test]
#[should_panic(expected = "not yet implemented")]
fn choice_skeleton_panics_only_when_executed() {
    run_choice_skeleton(0);
}
