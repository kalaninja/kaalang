use contour::contour;

#[contour]
fn invalid(condition: bool) -> u32 {
    #[question("Choose a value.")]
    |condition| -> (yes, no) { condition };

    #[action("Build a number.")]
    |yes| -> selected { 1_u32 };

    #[action("Build text.")]
    |no| -> selected { "no" };

    #[action("Use the selected value.")]
    |selected| -> result { 0 };

    #[end]
    |result| {};
}

fn main() {}
