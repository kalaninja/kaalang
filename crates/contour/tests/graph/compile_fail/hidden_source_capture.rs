use contour::contour;

#[contour]
fn invalid(input: u32, trigger: ()) -> u32 {
    #[action("Read an omitted source wire.")]
    |trigger| -> output { input };
}

fn main() {}
