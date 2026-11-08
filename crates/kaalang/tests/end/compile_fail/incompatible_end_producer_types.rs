use kaalang::kaa;

#[kaa]
fn invalid(condition: bool) -> u32 {
    #[kaa::question("Choose a result.")]
    let (yes, no) = |condition| condition;

    #[kaa::action("Build a number.")]
    let end = |yes| 1_u32;

    #[kaa::action("Build text.")]
    let end = |no| "no";

    |end| return end;
}

fn main() {}
