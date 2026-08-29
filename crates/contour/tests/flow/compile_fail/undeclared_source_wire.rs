use contour::contour;

#[contour]
fn invalid(input: u32, trigger: ()) -> u32 {
    #[action("Read a source wire omitted from the inputs.")]
    |trigger| -> output { input };
}

fn main() {}
