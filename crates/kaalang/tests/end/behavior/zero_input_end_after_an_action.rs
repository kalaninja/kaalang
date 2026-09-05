use kaalang::kaalang;

#[kaalang]
fn zero_input_end_after_an_action(input: u32, log: &mut Vec<u32>) {
    #[action("Log the input.")]
    |input, log| -> () { log.push(input) };

    #[end]
    || {};
}

#[test]
fn a_zero_input_end_waits_for_the_action() {
    let mut log = Vec::new();
    zero_input_end_after_an_action(3, &mut log);
    assert_eq!(log, [3]);
}
