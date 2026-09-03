use kaalang::kaalang;

#[kaalang]
fn capture_from_a_branch(condition: bool) {
    #[question("Is the short answer enough?")]
    |condition| -> (done, more) { condition };

    #[action("Work out the longer answer.")]
    |more| -> done {};

    #[end]
    |done| {};
}

#[test]
fn a_branch_may_reach_end_with_no_block_of_its_own() {
    capture_from_a_branch(true);
    capture_from_a_branch(false);
}
