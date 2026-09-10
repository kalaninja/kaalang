use kaalang::kaalang;

#[kaalang]
fn consume_a_borrowed_input_later(input: u32, log: &mut Vec<u32>) -> u32 {
    #[action("Record the input the flow returns.")]
    let logged = |&input, log| log.push(*input);

    #[action("Return the recorded input.")]
    let end = |input, logged| input;
}

#[test]
fn the_borrow_runs_above_the_block_that_takes_the_value() {
    let mut log = Vec::new();
    assert_eq!(consume_a_borrowed_input_later(7, &mut log), 7);
    assert_eq!(log, [7]);
}
