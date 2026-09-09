use kaalang::kaalang;

#[kaalang]
fn invalid(condition: bool) -> u32 {
    #[question("Choose a result.")]
    let (yes, no) = |condition| { condition };

    #[action("Build a number.")]
    let result = |yes| { 1_u32 };

    #[action("Build text.")]
    let result = |no| { "no" };
}

fn main() {}
