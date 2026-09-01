use contour::contour;

#[contour]
fn invalid(input: u32) -> u32 {
    #[merge]
    |input| -> result {};

    #[end]
    |result| {};
}

fn main() {}
