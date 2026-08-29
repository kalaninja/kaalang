use contour::contour;

#[contour]
fn unknown_attribute(value: i32) -> i32 {
    #[action("Double the value.")]
    #[unknown("Not a Contour attribute.")]
    |value| -> doubled { value * 2 };
}

fn main() {}
