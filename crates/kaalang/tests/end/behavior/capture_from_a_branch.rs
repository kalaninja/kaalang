use kaalang::kaalang;

#[kaalang]
fn capture_from_a_branch(condition: bool) {
    #[question("Is the short answer enough?")]
    let (result, more) = |condition| condition;

    #[action("Work out the longer answer.")]
    let result = |more| {};
}

#[test]
fn a_branch_may_reach_end_with_no_block_of_its_own() {
    capture_from_a_branch(true);
    capture_from_a_branch(false);
}
