use contour::contour;

#[contour]
fn invalid(input: u32) -> u32 {
    #[action("Prepare the value.")]
    |input| -> value { input };

    #[merge]
    |value| -> selected {};

    #[action("Return the selected value.")]
    |selected| -> result { selected };
}

fn main() {}
