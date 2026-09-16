use kaalang::kaalang;

struct Tally {
    total: u32,
}

impl Tally {
    #[kaalang]
    fn mutate_an_owned_receiver(mut self, step: u32) -> Self {
        #[action("Add the step to the total.")]
        let added = |self, step| {
            self.total += step;
            self
        };

        #[action("Double the total.")]
        let end = |added| Self {
            total: added.total * 2,
        };

        |end| return end;
    }
}

#[test]
fn a_value_receiver_declared_mut_is_mutated_in_place() {
    let tally = Tally { total: 1 };

    assert_eq!(tally.mutate_an_owned_receiver(2).total, 6);
}
