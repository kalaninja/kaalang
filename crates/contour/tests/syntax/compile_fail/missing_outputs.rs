use contour::contour;

#[contour]
fn invalid(input: u32) -> u32 {
    #[question("Omit the question outputs.")]
    |&input| true;
}

fn main() {}
