use contour::contour;

#[contour(unexpected)]
fn contour_arguments(value: i32) -> i32 {
    #[action("Double the value.")]
    |value| -> doubled { value * 2 };
}

fn main() {}
