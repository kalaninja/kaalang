use kaalang::kaalang;

#[kaalang]
fn consume_without_outputs(value: u32, other: u32, log: &mut Vec<u32>) -> u32 {
    #[action("Record the value and produce no wire.")]
    |value, log| -> () { log.push(value) };

    #[end]
    |other| {};
}

#[test]
fn an_action_may_consume_a_wire_and_produce_none() {
    let mut log = Vec::new();
    assert_eq!(consume_without_outputs(4, 9, &mut log), 9);
    assert_eq!(log, [4]);
}
