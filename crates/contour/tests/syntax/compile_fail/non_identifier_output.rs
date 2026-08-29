use contour::contour;

#[contour]
fn non_identifier_output(value: i32) -> Vec<i32> {
    #[action("Wrap the value.")]
    |value| -> Vec<i32> { vec![value] };
}

fn main() {}
