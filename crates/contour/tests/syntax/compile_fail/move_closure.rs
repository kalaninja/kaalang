use contour::contour;

#[contour]
fn move_closure(value: i32) -> i32 {
    #[action("Double the value.")]
    move |value| -> doubled { value * 2 };
}

fn main() {}
