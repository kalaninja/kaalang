use kaalang::kaalang;

#[kaalang]
fn consume_a_borrowed_input_later(input: u32, log: &mut Vec<u32>) -> u32 {
    #[action("Record the input the flow returns.")]
    |&input, log| -> logged { log.push(*input) };

    #[action("Return the recorded input.")]
    |input, logged| -> result { input };
}

#[test]
fn a_marker_wire_orders_the_borrow_before_the_consumer() {
    let mut log = Vec::new();
    assert_eq!(consume_a_borrowed_input_later(7, &mut log), 7);
    assert_eq!(log, [7]);
}
