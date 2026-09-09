use kaalang::kaalang;

#[kaalang]
fn mutably_borrowed_question_output(condition: bool) -> u8 {
    #[question("Select a branch.")]
    let (mut yes, no) = |condition| { condition };

    #[action("Try to borrow the selected branch output.")]
    let result = |&mut yes| { 1 };

    #[action("Use the other branch.")]
    let result = |no| { 0 };
}

fn main() {}
