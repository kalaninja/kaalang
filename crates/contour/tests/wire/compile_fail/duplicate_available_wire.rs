use contour::contour;

#[contour]
fn invalid(input: u32) -> u32 {
    #[action("Produce a wire.")]
    |input| -> shared { input };

    #[action("Produce the same available wire again.")]
    |&shared| -> shared { *shared };

    #[end]
    |shared| {};
}

fn main() {}
