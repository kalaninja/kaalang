use contour::contour;

#[contour]
fn reference_input(value: i32) -> i32 {
    #[action("Double the value.")]
    |ref value| -> doubled { *value * 2 };
}

fn main() {}
