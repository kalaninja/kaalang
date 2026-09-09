use kaalang::kaalang;

#[kaalang]
fn invalid(condition: bool) -> u32 {
    #[question("Choose a branch.")]
    |condition| -> (yes, no) { condition };

    #[action("Borrow the yes output.")]
    |&yes| -> result { 1 };

    #[action("Produce the no result.")]
    |no| -> result { 2 };
}

fn main() {}
