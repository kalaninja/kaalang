use contour::contour;

#[contour]
fn duplicate_source_wire(value: i32, value: i32) -> i32 {
    #[action("Double the value.")]
    |value| -> doubled { value * 2 };
}

fn main() {}
