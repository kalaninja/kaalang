use kaalang::kaalang;

#[kaalang]
fn invalid(condition: bool) -> u32 {
    #[question("Choose a branch.")]
    |condition| -> (yes, no) { condition };

    #[action("Build the result.")]
    |yes| -> result { 1 };

    #[action("Consume the no branch.")]
    |no| -> _ignored { () };
}

fn main() {}
