use contour::contour;

#[contour]
fn invalid(input: u32) -> u32 {
    #[action(Copy the input.)]
    |input| -> output { input };
}

fn main() {}
