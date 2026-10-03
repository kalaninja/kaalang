use kaalang::kaalang;

/// The outputs of one cycle merge into the gate of the next, and each cycle
/// keeps its own alternative outputs apart.
#[kaalang]
fn merged_outputs_gate_a_cycle(mut first: u32, mut second: u32) -> u32 {
    #[cycle("Count the first number past five.")]
    let (even, odd) = loop {
        #[question("Past five?")]
        let (past, again) = |&first| *first > 5;

        #[action("Count the first number up.")]
        let stepped = |again, &mut first| *first += 1;

        |stepped| continue;

        #[question("Is it even?")]
        let (even, odd) = |past, &first| *first % 2 == 0;
    };

    #[action("Start from one.")]
    let base = |even| 1;

    #[action("Start from two.")]
    let base = |odd| 2;

    #[cycle("Count the second number past three.")]
    let (large, small) = |base| loop {
        #[question("Past three?")]
        let (past, again) = |&second| *second > 3;

        #[action("Count the second number up.")]
        let stepped = |again, &mut second| *second += 1;

        |stepped| continue;

        #[question("Is it large?")]
        let (large, small) = |past, &second| *second > 10;
    };

    #[action("Add three.")]
    let result = |large, base| base + 3;

    #[action("Add two.")]
    let result = |small, base| base + 2;

    |result| return result;
}

#[test]
fn the_second_cycle_starts_from_the_first_ones_merged_result() {
    assert_eq!(merged_outputs_gate_a_cycle(0, 0), 3);
    assert_eq!(merged_outputs_gate_a_cycle(7, 20), 5);
}
