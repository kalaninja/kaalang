#[kaalang]
fn route(input: u8) -> u8 {
    #[choice("Which path does the input take?")]
    #[case("Take the left path.")]
    #[case("Take the right path.")]
    #[case("Take the first direct End path.")]
    #[case("Take the second direct End path.")]
    |input| -> (left, right, first_direct, second_direct) {
        match input {
            0 => (),
            1 => (),
            2 => (),
            _ => (),
        }
    };

    #[action("Build the left value.")]
    |left| -> selected { 1 };

    #[action("Build the first direct result.")]
    |first_direct| -> result { 2 };

    #[action("Build the second direct result.")]
    |second_direct| -> result { 3 };

    #[action("Build the right value.")]
    |right| -> selected { 4 };

    #[action("Build the converged result.")]
    |selected| -> result { selected };

    #[end]
    |result| {};
}
