use kaalang::kaalang;

// Every branch notes its answer and every combination reaches the result, but
// the second question opens its own branches while the first question's are
// still separate, which is the only way two selections could decide one block.
#[kaalang]
fn invalid(left: bool, right: bool) -> u8 {
    #[question("Left enabled?")]
    let (left_yes, left_no) = |left| { left };

    #[question("Right enabled?")]
    let (right_yes, right_no) = |right| { right };

    #[action("Note the left answer.")]
    let left_note = |left_yes| { 1u8 };

    #[action("Note the missing left answer.")]
    let left_absent = |left_no| { 0u8 };

    #[action("Note the right answer.")]
    let right_note = |right_yes| { 2u8 };

    #[action("Note the missing right answer.")]
    let right_absent = |right_no| { 0u8 };

    #[action("Use both notes.")]
    let end = |left_note, right_note| { left_note + right_note };

    #[action("Use the left note only.")]
    let end = |left_note, right_absent| { left_note + right_absent };

    #[action("Use the right note only.")]
    let end = |left_absent, right_note| { left_absent + right_note };

    #[action("Use neither note.")]
    let end = |left_absent, right_absent| { left_absent + right_absent };
}

fn main() {}
