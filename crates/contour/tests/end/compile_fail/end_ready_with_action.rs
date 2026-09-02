use contour::contour;

#[contour]
fn invalid(input: u32) -> u32 {
    #[action("Run an unrelated action.")]
    |&input| -> _ignored { () };

    #[end]
    |input| {};
}

fn main() {}
