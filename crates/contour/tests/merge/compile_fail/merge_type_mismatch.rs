use contour::contour;

#[contour]
fn invalid(condition: bool) -> u32 {
    #[question("Select a branch.")]
    |condition| -> (yes, no) { condition };

    #[action("Build a number.")]
    |yes| -> number { 1_u32 };

    #[action("Build text.")]
    |no| -> text { "no" };

    #[merge]
    |number, text| -> selected {};

    #[action("Return the selected value.")]
    |selected| -> result { selected };
}

fn main() {}
