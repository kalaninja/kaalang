use kaalang::kaalang;

#[kaalang]
fn invalid(condition: bool) -> u32 {
    #[question("Choose a result.")]
    let (yes, no) = |condition| condition;

    #[action("Build a number.")]
    let end = |yes| 1_u32;

    #[action("Build text.")]
    let end = |no| "no";

    |end| return end;
}

fn main() {}
