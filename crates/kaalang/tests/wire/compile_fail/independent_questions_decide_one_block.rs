use kaalang::kaalang;

// Every branch notes its answer and every combination reaches the result, but
// the second question opens its own branches while the first question's are
// still separate, which is the only way two selections could decide one block.
#[kaalang]
fn invalid(left: bool, right: bool) -> u8 {
    #[question("Left enabled?")]
    |left| -> (left_yes, left_no) { left };

    #[question("Right enabled?")]
    |right| -> (right_yes, right_no) { right };

    #[action("Note the left answer.")]
    |left_yes| -> left_note { 1u8 };

    #[action("Note the missing left answer.")]
    |left_no| -> left_absent { 0u8 };

    #[action("Note the right answer.")]
    |right_yes| -> right_note { 2u8 };

    #[action("Note the missing right answer.")]
    |right_no| -> right_absent { 0u8 };

    #[action("Use both notes.")]
    |left_note, right_note| -> result { left_note + right_note };

    #[action("Use the left note only.")]
    |left_note, right_absent| -> result { left_note + right_absent };

    #[action("Use the right note only.")]
    |left_absent, right_note| -> result { left_absent + right_note };

    #[action("Use neither note.")]
    |left_absent, right_absent| -> result { left_absent + right_absent };
}

fn main() {}
