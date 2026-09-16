use kaalang::kaalang;

struct Message {
    text: String,
}

impl Message {
    #[kaalang]
    fn consume_a_receiver(self, suffix: String) -> String {
        #[action("Append the suffix to the text.")]
        let end = |self, suffix| self.text + &suffix;

        |end| return end;
    }
}

#[test]
fn a_receiver_taken_by_value_moves_into_its_consumer() {
    let message = Message {
        text: "kaa".to_owned(),
    };

    assert_eq!(message.consume_a_receiver("lang".to_owned()), "kaalang");
}
