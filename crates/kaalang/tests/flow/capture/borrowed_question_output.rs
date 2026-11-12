use kaalang::kaa;

/// A question output may be borrowed. The borrower is still the branch's
/// continuation, so the branch is entered exactly once.
#[kaa]
fn borrowed_question_output(condition: bool) -> u32 {
    #[kaa::question("Choose a branch.")]
    let (yes, no) = |condition| condition;

    #[kaa::action("Borrow the yes output.")]
    let end = |&yes| 1;

    #[kaa::action("Produce the no result.")]
    let end = |no| 2;

    |end| return end;
}

#[test]
fn a_borrowed_question_output_enters_its_branch() {
    assert_eq!(borrowed_question_output(true), 1);
    assert_eq!(borrowed_question_output(false), 2);
}
