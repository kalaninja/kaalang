use kaalang::kaalang;

#[kaalang]
fn invalid(condition: bool) -> u32 {
    #[question("Choose a branch.")]
    let (yes, no) = |condition| { condition };

    #[action("Borrow the yes output.")]
    let result = |&yes| { 1 };

    #[action("Produce the no result.")]
    let result = |no| { 2 };
}

fn main() {}
