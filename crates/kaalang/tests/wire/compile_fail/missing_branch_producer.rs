use kaalang::kaalang;

#[kaalang]
fn invalid(condition: bool) -> u32 {
    #[question("Choose a branch.")]
    let (yes, no) = |condition| { condition };

    #[action("Build both required values.")]
    let (selected, continuation) = |yes| { (1, ()) };

    #[action("Build only one required value.")]
    let continuation = |no| { () };

    #[action("Use both values.")]
    let end = |selected, continuation| { selected };

    |end| return end;
}

fn main() {}
