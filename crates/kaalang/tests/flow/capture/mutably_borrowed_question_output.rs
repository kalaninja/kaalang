#![deny(unused_mut)]

use kaalang::kaa;

/// A question output declared `mut` may be borrowed mutably.
#[kaa]
fn mutably_borrowed_question_output(condition: bool) -> u8 {
    #[kaa::question("Select a branch.")]
    let (mut yes, no) = |condition| condition;

    #[kaa::action("Mutably borrow the selected branch output.")]
    let end = |&mut yes| {
        *yes = ();
        1
    };

    #[kaa::action("Use the other branch.")]
    let end = |no| 0;

    |end| return end;
}

#[test]
fn a_question_output_can_be_borrowed_mutably() {
    assert_eq!(mutably_borrowed_question_output(true), 1);
    assert_eq!(mutably_borrowed_question_output(false), 0);
}
