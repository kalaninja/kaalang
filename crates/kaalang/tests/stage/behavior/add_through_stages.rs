use kaalang::kaa;

struct Counter {
    total: usize,
}

impl Counter {
    #[kaa]
    fn add_through_stages(&mut self, value: usize) -> usize {
        #[kaa::action("Begin adding.")]
        let add = || {};

        #[kaa::stage("Add the value.")]
        let finish = |add| {
            #[kaa::action("Update the receiver.")]
            let finish = |&mut self, value| {
                self.total += value;
            };
        };

        #[kaa::stage("Read the total.")]
        |finish| {
            #[kaa::action("Read the updated total.")]
            let total = |&mut self| self.total;

            |total| return total;
        };
    }
}

#[test]
fn stage_blocks_can_capture_a_mutable_receiver() {
    let mut counter = Counter { total: 4 };
    assert_eq!(counter.add_through_stages(3), 7);
    assert_eq!(counter.total, 7);
}
