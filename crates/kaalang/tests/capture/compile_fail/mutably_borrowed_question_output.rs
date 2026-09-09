use kaalang::kaalang;

#[kaalang]
fn mutably_borrowed_question_output(condition: bool) -> u8 {
    #[question("Select a branch.")]
    |condition| -> (yes, no) { condition };

    #[action("Try to borrow the selected branch output.")]
    |&mut yes| -> result { 1 };

    #[action("Use the other branch.")]
    |no| -> result { 0 };
}

fn main() {}
