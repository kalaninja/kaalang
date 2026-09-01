use contour::contour;

#[contour]
fn invalid(input: u32) -> u32 {
    #[action("Produce the first result.")]
    |&input| -> first { *input };

    #[action("Produce the second result.")]
    |&input| -> second { *input };

    #[end]
    |first, second| {};
}

fn main() {}
