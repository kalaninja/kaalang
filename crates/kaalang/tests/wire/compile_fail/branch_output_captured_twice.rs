use kaalang::kaalang;

#[kaalang]
fn invalid(condition: bool) -> u8 {
    #[question("Choose the branch.")]
    let (yes, no) = |condition| { condition };

    #[action("Start the yes branch.")]
    let first = |yes| { 1u8 };

    #[action("Start the yes branch a second time.")]
    let second = |yes| { 2u8 };

    #[action("Finish the yes branch.")]
    let end = |first, second| { first + second };

    #[action("Finish the no branch.")]
    let end = |no| { 0u8 };
}

fn main() {}
