use contour::contour;

#[contour]
fn invalid(value: u8) -> u8 {
    #[choice("Select a branch.")]
    #[case("Take the first branch.")]
    #[case("Finish without merging.")]
    #[case("Take the second branch.")]
    |value| -> (first, done, second) {
        match value {
            0 => (),
            1 => (),
            _ => (),
        }
    };

    #[action("Build the first value.")]
    |first| -> first_value { 1 };

    #[action("Finish immediately.")]
    |done| -> done_result { 2 };

    #[action("Build the second value.")]
    |second| -> second_value { 3 };

    #[merge]
    |first_value, second_value| -> selected {};

    #[action("Return the merged value.")]
    |selected| -> merged_result { selected };
}

fn main() {}
