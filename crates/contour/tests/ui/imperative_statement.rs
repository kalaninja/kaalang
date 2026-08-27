use contour::contour;

#[contour]
fn invalid(input: u32) -> u32 {
    // This is an ordinary comment, not a block description.
    let output = input;
}

fn main() {}
