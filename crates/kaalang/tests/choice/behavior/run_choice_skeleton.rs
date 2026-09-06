use kaalang::kaalang;

#[allow(unreachable_code)]
#[kaalang]
fn run_choice_skeleton(value: i32) -> &'static str {
    #[choice("What is the sign of the value?")]
    #[case("The value is negative.")]
    #[case("The value is nonnegative.")]
    |value| -> (negative, nonnegative) { todo!() };

    #[action("Produce the negative result.")]
    |negative| -> result { todo!() };

    #[action("Produce the nonnegative result.")]
    |nonnegative| -> result { todo!() };
}

#[test]
#[should_panic(expected = "not yet implemented")]
fn choice_skeleton_panics_only_when_executed() {
    run_choice_skeleton(0);
}
