use kaalang::kaalang;

struct Rectangle {
    width: u32,
    height: u32,
}

impl Rectangle {
    #[kaalang]
    fn read_a_borrowed_receiver(&self, scale: u32) -> u32 {
        #[action("Scale the width.")]
        let scaled = |&self, scale| self.width * scale;

        #[action("Add the height.")]
        let end = |&self, scaled| scaled + self.height;

        |end| return end;
    }
}

#[test]
fn a_shared_receiver_reaches_every_block_that_captures_it() {
    let rectangle = Rectangle {
        width: 3,
        height: 4,
    };

    assert_eq!(rectangle.read_a_borrowed_receiver(2), 10);
}
