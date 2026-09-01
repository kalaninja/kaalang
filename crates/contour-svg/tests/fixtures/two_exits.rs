#[contour]
fn route(input: u8) -> u8 {
    #[choice("Which path does the input take?")]
    #[case("Take the left path.")]
    #[case("Take the right path.")]
    #[case("Finish at the first exit.")]
    #[case("Finish at the second exit.")]
    |input| -> (left, right, first_exit, second_exit) {
        match input {
            0 => (),
            1 => (),
            2 => (),
            _ => (),
        }
    };

    #[action("Build the left value.")]
    |left| -> selected { 1 };

    #[action("Finish immediately at the first exit.")]
    |first_exit| -> result { 2 };

    #[action("Finish immediately at the second exit.")]
    |second_exit| -> result { 3 };

    #[action("Build the right value.")]
    |right| -> selected { 4 };

    #[action("Finish after convergence.")]
    |selected| -> result { selected };

    #[end]
    |result| {};
}
