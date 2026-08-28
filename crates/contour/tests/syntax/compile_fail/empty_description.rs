#[contour::contour]
fn invalid(input: u32) -> u32 {
    #[action("")]
    |input| -> output { input };
}

fn main() {}
