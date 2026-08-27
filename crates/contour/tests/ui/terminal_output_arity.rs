use contour::contour;

#[contour]
fn invalid(input: u32) -> u32 {
    #[action("Split the input but leave both outputs unconsumed.")]
    |&input| -> (left, right) { (*input, *input + 1) };
}

fn main() {}
