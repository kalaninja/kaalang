use kaalang::kaalang;

#[kaalang]
fn invalid(condition: bool) -> u32 {
    #[question("Choose a result.")]
    |condition| -> (yes, no) { condition };

    #[action("Build a number.")]
    |yes| -> result { 1_u32 };

    #[action("Build text.")]
    |no| -> result { "no" };
}

fn main() {}
