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
    |left| -> left_value { 1 };

    #[action("Finish immediately at the first exit.")]
    |first_exit| -> first_result { 2 };

    #[action("Finish immediately at the second exit.")]
    |second_exit| -> second_result { 3 };

    #[action("Build the right value.")]
    |right| -> right_value { 4 };

    #[merge]
    |left_value, right_value| -> selected {};

    #[action("Finish after the merge.")]
    |selected| -> merged_result { selected };
}
