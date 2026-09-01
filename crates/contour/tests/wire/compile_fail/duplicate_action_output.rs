use contour::contour;

#[contour]
fn invalid(input: u32) -> (u32, u32) {
    #[action("Produce two outputs with one name.")]
    |input| -> (shared, shared) { (input, input) };

    #[end]
    |shared| {};
}

fn main() {}
