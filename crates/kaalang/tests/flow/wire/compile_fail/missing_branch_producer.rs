use kaalang::kaa;

#[kaa]
fn invalid(condition: bool) -> u32 {
    #[kaa::question("Choose a branch.")]
    let (yes, no) = |condition| { condition };

    #[kaa::action("Build both required values.")]
    let (selected, continuation) = |yes| { (1, ()) };

    #[kaa::action("Build only one required value.")]
    let continuation = |no| { () };

    #[kaa::action("Use both values.")]
    let end = |selected, continuation| { selected };

    |end| return end;
}

fn main() {}
