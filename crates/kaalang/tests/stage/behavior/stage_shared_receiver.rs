use kaalang::kaalang;

struct Counter(u8);

impl Counter {
    #[kaalang]
    fn stage_shared_receiver(&self, go: ()) -> u8 {
        #[stage("Read the shared receiver.")]
        let finish = |go| {
            #[action("Borrow the receiver's value.")]
            let finish = |&self, go| self.0;
        };

        #[stage("Return the receiver's value.")]
        |finish| {
            |finish| return finish;
        };
    }
}

#[test]
fn a_shared_receiver_can_be_captured_in_a_stage() {
    assert_eq!(Counter(7).stage_shared_receiver(()), 7);
}
