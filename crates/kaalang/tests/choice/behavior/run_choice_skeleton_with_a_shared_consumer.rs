use kaalang::kaa;

#[allow(unreachable_code)]
#[kaa]
fn run_choice_skeleton_with_a_shared_consumer(value: i32) -> i32 {
    #[kaa::choice("What is the sign of the value?")]
    #[kaa::case("The value is negative.")]
    #[kaa::case("The value is nonnegative.")]
    let (negative, nonnegative) = |value| todo!();

    #[kaa::action("Build the negative magnitude.")]
    let magnitude = |negative| todo!();

    #[kaa::action("Build the nonnegative magnitude.")]
    let magnitude = |nonnegative| todo!();

    |magnitude| return magnitude;
}

#[test]
#[should_panic(expected = "not yet implemented")]
fn a_choice_placeholder_still_binds_its_join() {
    run_choice_skeleton_with_a_shared_consumer(0);
}
