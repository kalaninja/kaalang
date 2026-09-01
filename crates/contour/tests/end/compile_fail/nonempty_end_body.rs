use contour::contour;

#[contour]
fn invalid(input: u32) -> u32 {
    #[end]
    |input| { input };
}

fn main() {}
