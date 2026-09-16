use kaalang::kaalang;

struct Rectangle {
    width: u32,
}

fn area(rectangle: &Rectangle, scale: u32) -> u32 {
    rectangle.width * scale
}

impl Rectangle {
    #[kaalang]
    fn call_with_the_receiver(&self, scale: u32) -> u32 {
        #[call("Compute the scaled area.")]
        let end = |&self, scale| area(self, scale);

        |end| return end;
    }
}

#[test]
fn a_call_passes_the_receiver_as_an_argument() {
    let rectangle = Rectangle { width: 4 };

    assert_eq!(rectangle.call_with_the_receiver(3), 12);
}
