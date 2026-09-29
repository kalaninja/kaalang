use kaalang::kaalang;

struct Counter(u8);

impl Counter {
    #[kaalang]
    fn stage_owned_receiver(self, go: ()) -> u8 {
        #[stage("Read the owned receiver.")]
        let finish = |go| {
            #[action("Take the receiver's value.")]
            let finish = |self, go| self.0;
        };

        #[stage("Return the receiver's value.")]
        |finish| {
            |finish| return finish;
        };
    }
}

#[test]
fn an_owned_receiver_moves_into_a_stage() {
    assert_eq!(Counter(7).stage_owned_receiver(()), 7);
}
