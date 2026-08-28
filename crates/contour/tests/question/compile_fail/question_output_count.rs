use contour::contour;

#[contour]
fn invalid(input: u32) -> u32 {
    #[question("Ask a question with one output.")]
    |&input| -> yes { true };
}

fn main() {}
