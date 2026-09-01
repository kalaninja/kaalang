use contour::contour;

#[contour]
fn invalid(condition: bool) -> u32 {
    #[question("Choose a path.")]
    |condition| -> (yes, no) { condition };

    #[action("Build both required values.")]
    |yes| -> (selected, continuation) { (1, ()) };

    #[action("Build only one required value.")]
    |no| -> continuation { () };

    #[action("Use both values.")]
    |selected, continuation| -> result { selected };
}

fn main() {}
