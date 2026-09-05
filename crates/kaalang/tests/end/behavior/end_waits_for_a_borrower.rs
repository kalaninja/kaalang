use kaalang::kaalang;

#[kaalang]
fn end_waits_for_a_borrower(input: u32, log: &mut Vec<u32>) -> u32 {
    #[action("Record the input the flow returns.")]
    |&input, log| -> () { log.push(*input) };

    #[end]
    |input| {};
}

#[test]
fn end_is_ready_only_after_the_borrower_ran() {
    let mut log = Vec::new();
    assert_eq!(end_waits_for_a_borrower(7, &mut log), 7);
    assert_eq!(log, [7]);
}
