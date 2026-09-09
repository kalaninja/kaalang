use kaalang::kaalang;

#[kaalang]
fn invalid(condition: bool) -> u8 {
    #[question("Choose the branch.")]
    |condition| -> (yes, no) { condition };

    #[action("Start the yes branch.")]
    |yes| -> first { 1u8 };

    #[action("Start the yes branch a second time.")]
    |yes| -> second { 2u8 };

    #[action("Finish the yes branch.")]
    |first, second| -> result { first + second };

    #[action("Finish the no branch.")]
    |no| -> result { 0u8 };
}

fn main() {}
