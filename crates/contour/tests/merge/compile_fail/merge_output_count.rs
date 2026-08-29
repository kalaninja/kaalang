use contour::contour;

#[contour]
fn invalid(condition: bool) -> bool {
    #[question("Select a branch.")]
    |condition| -> (yes, no) { condition };

    #[action("Build the yes value.")]
    |yes| -> yes_value { true };

    #[action("Build the no value.")]
    |no| -> no_value { false };

    #[merge]
    |yes_value, no_value| -> (selected,) {};

    #[action("Return the value.")]
    |selected| -> result { selected };
}

fn main() {}
