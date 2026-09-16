use kaalang::kaalang;

struct Counter {
    value: u32,
}

impl Counter {
    #[kaalang]
    fn mutate_a_receiver(&mut self, step: u32) -> u32 {
        #[action("Advance the counter.")]
        let advanced = |&mut self, step| {
            self.value += step;
        };

        #[action("Read the counter.")]
        let end = |advanced, &mut self| self.value;

        |end| return end;
    }
}

#[test]
fn a_mutable_receiver_is_reborrowed_by_each_block() {
    let mut counter = Counter { value: 1 };

    assert_eq!(counter.mutate_a_receiver(4), 5);
    assert_eq!(counter.value, 5);
}
