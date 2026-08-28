use contour::contour;

#[contour]
fn invalid(input: String) -> usize {
    #[action("Move out of a borrowed input.")]
    |&input| -> output { input.into_bytes().len() };
}

fn main() {}
