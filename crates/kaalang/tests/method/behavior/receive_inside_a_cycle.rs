use kaalang::kaalang;

struct Ladder {
    top: u32,
}

impl Ladder {
    #[kaalang]
    fn receive_inside_a_cycle(&self, start: u32) -> u32 {
        #[cycle("Climb to the top.")]
        let reached = |&self, mut start| {
            #[question("Is the rung below the top?")]
            #[yes("YES")]
            #[no("NO")]
            let (climb, arrived) = |&start, &self| *start < self.top;

            |arrived, start| break start;

            #[action("Step up one rung.")]
            |climb, &mut start| *start += 1;
        };

        |reached| return reached;
    }
}

#[test]
fn a_receiver_is_read_on_every_iteration() {
    let ladder = Ladder { top: 3 };

    assert_eq!(ladder.receive_inside_a_cycle(0), 3);
    assert_eq!(ladder.receive_inside_a_cycle(7), 7);
}
