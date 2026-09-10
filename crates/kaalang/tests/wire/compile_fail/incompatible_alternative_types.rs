use kaalang::kaalang;

#[kaalang]
fn invalid(condition: bool) -> u32 {
    #[question("Choose a value.")]
    let (yes, no) = |condition| { condition };

    #[action("Build a number.")]
    let selected = |yes| { 1_u32 };

    #[action("Build text.")]
    let selected = |no| { "no" };

    #[action("Use the selected value.")]
    let end = |selected| { 0 };
}

fn main() {}
