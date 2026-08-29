use contour::contour;

#[contour]
fn invalid(input: u32) -> String {
    #[action("Return a number from a string flow.")]
    |&input| -> output { *input };
}

fn main() {}
