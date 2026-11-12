use kaalang::kaa;

trait Measure {
    fn width(&self) -> u32;

    #[kaa]
    fn doubled(&self) -> u32 {
        #[kaa::action("Double the width.")]
        let end = |&self| self.width() * 2;

        |end| return end;
    }
}

struct Text;

impl Measure for Text {
    fn width(&self) -> u32 {
        3
    }
}

#[test]
fn a_trait_default_body_declares_a_flow() {
    assert_eq!(Text.doubled(), 6);
}
