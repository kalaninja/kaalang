use kaalang::kaalang;

#[kaalang]
fn invalid(condition: bool) -> u32 {
    #[question("Choose a value.")]
    let (yes, no) = |condition| { condition };

    #[action("Build the first value.")]
    let selected = |yes| { 1 };

    |selected| return selected;

    #[action("Build the later alternative.")]
    let selected = |no| { 2 };

}

fn main() {}
