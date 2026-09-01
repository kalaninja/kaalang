use contour::contour;

#[contour]
fn invalid(value: u8) -> u8 {
    #[choice("Select a branch.")]
    #[case("Take the first branch.")]
    #[case("Reach End without converging.")]
    #[case("Take the second branch.")]
    |value| -> (first, done, second) {
        match value {
            0 => (),
            1 => (),
            _ => (),
        }
    };

    #[action("Build the first value.")]
    |first| -> selected { 1 };

    #[action("Produce the direct result.")]
    |done| -> result { 2 };

    #[action("Build the second value.")]
    |second| -> selected { 3 };

    #[action("Produce the selected result.")]
    |selected| -> result { selected };

    #[end]
    |result| {};
}

fn main() {}
