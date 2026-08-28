#[contour::contour]
fn invalid(input: u32) -> u32 {
    // A comment is deliberately not the block description.
    #[action]
    |input| -> output { input };
}

fn main() {}
