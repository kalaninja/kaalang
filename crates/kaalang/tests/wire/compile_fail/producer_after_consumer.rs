use kaalang::kaa;

#[kaa]
fn invalid(condition: bool) -> u32 {
    #[kaa::question("Choose a value.")]
    let (yes, no) = |condition| { condition };

    #[kaa::action("Build the first value.")]
    let selected = |yes| { 1 };

    |selected| return selected;

    #[kaa::action("Build the later alternative.")]
    let selected = |no| { 2 };

}

fn main() {}
