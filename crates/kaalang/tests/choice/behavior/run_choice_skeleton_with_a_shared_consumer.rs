use kaalang::kaalang;

#[allow(unreachable_code)]
#[kaalang]
fn run_choice_skeleton_with_a_shared_consumer(value: i32) -> i32 {
    #[choice("What is the sign of the value?")]
    #[case("The value is negative.")]
    #[case("The value is nonnegative.")]
    |value| -> (negative, nonnegative) { todo!() };

    #[action("Build the negative magnitude.")]
    |negative| -> magnitude { todo!() };

    #[action("Build the nonnegative magnitude.")]
    |nonnegative| -> magnitude { todo!() };

    #[action("Use the selected magnitude.")]
    |magnitude| -> result { magnitude };
}

#[test]
#[should_panic(expected = "not yet implemented")]
fn a_choice_placeholder_still_binds_its_join() {
    run_choice_skeleton_with_a_shared_consumer(0);
}
