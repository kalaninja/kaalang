use kaalang::kaalang;

trait Measure {
    fn measure(&self) -> usize;
}

struct Text(String);

impl Measure for Text {
    #[kaalang]
    fn measure(&self) -> usize {
        #[action("Measure the text.")]
        let end = |&self| self.0.len();

        |end| return end;
    }
}

#[test]
fn a_trait_method_declares_a_flow() {
    assert_eq!(Text("kaalang".to_owned()).measure(), 7);
}
