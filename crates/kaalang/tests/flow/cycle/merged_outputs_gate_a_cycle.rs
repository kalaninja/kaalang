use kaalang::kaa;

/// The outputs of one cycle merge into the gate of the next, and each cycle
/// keeps its own alternative outputs apart.
#[kaa]
fn merged_outputs_gate_a_cycle(mut first: u32, mut second: u32) -> u32 {
    #[kaa::cycle("Count the first number past five.")]
    let (even, odd) = loop {
        #[kaa::question("Past five?")]
        let (past, again) = |&first| *first > 5;

        #[kaa::action("Count the first number up.")]
        let stepped = |again, &mut first| *first += 1;

        |stepped| continue;

        #[kaa::question("Is it even?")]
        let (even, odd) = |past, &first| *first % 2 == 0;
    };

    #[kaa::action("Start from one.")]
    let base = |even| 1;

    #[kaa::action("Start from two.")]
    let base = |odd| 2;

    #[kaa::cycle("Count the second number past three.")]
    let (large, small) = |base| loop {
        #[kaa::question("Past three?")]
        let (past, again) = |&second| *second > 3;

        #[kaa::action("Count the second number up.")]
        let stepped = |again, &mut second| *second += 1;

        |stepped| continue;

        #[kaa::question("Is it large?")]
        let (large, small) = |past, &second| *second > 10;
    };

    #[kaa::action("Add three.")]
    let result = |large, base| base + 3;

    #[kaa::action("Add two.")]
    let result = |small, base| base + 2;

    |result| return result;
}

#[test]
fn the_second_cycle_starts_from_the_first_ones_merged_result() {
    assert_eq!(merged_outputs_gate_a_cycle(0, 0), 3);
    assert_eq!(merged_outputs_gate_a_cycle(7, 20), 5);
}
