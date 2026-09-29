use kaalang::kaalang;

struct Counter {
    total: usize,
}

impl Counter {
    #[kaalang]
    fn add_through_stages(&mut self, value: usize) -> usize {
        #[action("Begin adding.")]
        let add = || {};

        #[stage("Add the value.")]
        let finish = |add| {
            #[action("Update the receiver.")]
            let finish = |&mut self, value| {
                self.total += value;
            };
        };

        #[stage("Read the total.")]
        |finish| {
            #[action("Read the updated total.")]
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
