use contour::contour;

#[contour]
fn invalid(condition: bool) -> u32 {
    #[question("Choose a value.")]
    |condition| -> (yes, no) { condition };

    #[action("Build the yes values.")]
    |yes| -> (number, _log) { (1, "yes") };

    #[action("Build the no values.")]
    |no| -> (number, _log) { (2, 3u8) };

    #[action("Use the number.")]
    |number| -> result { number };

    #[end]
    |result| {};
}

fn main() {}
