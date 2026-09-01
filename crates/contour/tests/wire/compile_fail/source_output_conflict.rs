use contour::contour;

#[contour]
fn invalid(input: u32) -> u32 {
    #[action("Reuse the source name as an output.")]
    |&input| -> input { *input };
}

fn main() {}
