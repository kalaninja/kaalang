use kaalang::kaalang;

#[kaalang]
fn end_after_an_action(input: u32, log: &mut Vec<u32>) {
    #[action("Log the input.")]
    |input, log| -> logged { log.push(input) };

    #[action("Finish once the input is logged.")]
    |logged| -> result { logged };
}

#[test]
fn end_waits_for_the_action_through_its_wire() {
    let mut log = Vec::new();
    end_after_an_action(3, &mut log);
    assert_eq!(log, [3]);
}
