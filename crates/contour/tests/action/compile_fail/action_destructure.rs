use contour::contour;

#[contour]
fn invalid(input: u32) -> u32 {
    #[action("Return one value.")]
    |&input| -> (left, right) { *input };

    #[action("Add both outputs.")]
    |left, right| -> output { left + right };

    #[end]
    |output| {};
}

fn main() {}
