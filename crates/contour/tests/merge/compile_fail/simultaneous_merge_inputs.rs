use contour::contour;

#[contour]
fn invalid(condition: bool) -> u32 {
    #[question("Select a branch.")]
    |condition| -> (yes, no) { condition };

    #[action("Build two values on one path.")]
    |yes| -> (left, right) { (1_u32, 2_u32) };

    #[action("Return the other path.")]
    |no| -> other { 0 };

    #[merge]
    |left, right| -> selected {};

    #[action("Return the selected value.")]
    |selected| -> result { selected };
}

fn main() {}
