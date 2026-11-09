use kaalang::kaa;

struct Gate {
    limit: u32,
}

impl Gate {
    #[kaa]
    fn receive_inside_a_branch(&self, value: u32) -> u32 {
        #[kaa::question("Is the value above the limit?")]
        #[kaa::yes("YES")]
        #[kaa::no("NO")]
        let (above, below) = |&self, &value| *value > self.limit;

        #[kaa::action("Clamp the value to the limit.")]
        let end = |above, &self| self.limit;

        #[kaa::action("Keep the value.")]
        let end = |below, value| value;

        |end| return end;
    }
}

#[test]
fn a_receiver_reaches_one_branch_only() {
    let gate = Gate { limit: 5 };

    assert_eq!(gate.receive_inside_a_branch(9), 5);
    assert_eq!(gate.receive_inside_a_branch(2), 2);
}
