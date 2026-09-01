use contour::contour;

#[contour]
fn invalid(condition: bool) -> u32 {
    #[question("Choose a path.")]
    |condition| -> (yes, no) { condition };

    #[action("Build the result.")]
    |yes| -> result { 1 };

    #[action("Consume the no branch.")]
    |no| -> _ignored { () };

    #[end]
    |result| {};
}

fn main() {}
