use kaalang::kaalang;

#[kaalang]
fn captured_in_one_branch(condition: bool, extra: u32) -> u32 {
    #[question("Use the extra value?")]
    |condition| -> (yes, no) { condition };

    #[action("Prepare a token that only one branch needs.")]
    || -> token { 5 };

    #[action("Combine the extra value with the token.")]
    |yes, extra, token| -> result { extra + token };

    #[action("Ignore both.")]
    |no| -> result { 0 };

    #[end]
    |result| {};
}

#[test]
fn a_producer_captured_in_one_execution_is_valid() {
    assert_eq!(captured_in_one_branch(true, 3), 8);
    assert_eq!(captured_in_one_branch(false, 3), 0);
}
