use kaalang::kaalang;

#[allow(unreachable_code)]
#[kaalang]
fn run_choice_skeleton(value: i32) -> &'static str {
    #[choice("What is the sign of the value?")]
    #[case("The value is negative.")]
    #[case("The value is nonnegative.")]
    let (negative, nonnegative) = |value| todo!();

    #[action("Produce the negative result.")]
    let end = |negative| todo!();

    #[action("Produce the nonnegative result.")]
    let end = |nonnegative| todo!();
}

#[test]
#[should_panic(expected = "not yet implemented")]
fn choice_skeleton_panics_only_when_executed() {
    run_choice_skeleton(0);
}
