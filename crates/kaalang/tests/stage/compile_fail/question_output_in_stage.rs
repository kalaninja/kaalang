use kaalang::kaalang;

#[kaalang]
fn invalid(take: bool) {
    #[question("Enter the stage?")]
    let (yes, no) = |take| take;

    #[action("Enter the stage.")]
    let go = |yes| ();

    #[cycle("Stay on the other branch.")]
    |no| loop {
        continue;
    };

    #[stage("Use the unmerged question output.")]
    |go| {
        |yes| return;
    };
}

fn main() {}
