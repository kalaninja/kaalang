use kaalang::kaa;

#[kaa]
fn invalid(condition: bool) -> u32 {
    #[kaa::question("Choose a value.")]
    let (yes, no) = |condition| { condition };

    #[kaa::action("Build a number.")]
    let selected = |yes| { 1_u32 };

    #[kaa::action("Build text.")]
    let selected = |no| { "no" };

    #[kaa::action("Use the selected value.")]
    let end = |selected| { 0 };

    |end| return end;
}

fn main() {}
