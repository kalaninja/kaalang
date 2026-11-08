use kaalang::kaa;

struct Counter(u8);

impl Counter {
    #[kaa]
    fn stage_owned_receiver(self, go: ()) -> u8 {
        #[kaa::stage("Read the owned receiver.")]
        let finish = |go| {
            #[kaa::action("Take the receiver's value.")]
            let finish = |self, go| self.0;
        };

        #[kaa::stage("Return the receiver's value.")]
        |finish| {
            |finish| return finish;
        };
    }
}

#[test]
fn an_owned_receiver_moves_into_a_stage() {
    assert_eq!(Counter(7).stage_owned_receiver(()), 7);
}
