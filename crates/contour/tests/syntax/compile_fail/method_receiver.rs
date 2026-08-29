use contour::contour;

struct Calculator;

impl Calculator {
    #[contour]
    fn method(&self, value: i32) -> i32 {
        #[action("Double the value.")]
        |value| -> doubled { value * 2 };
    }
}

fn main() {}
