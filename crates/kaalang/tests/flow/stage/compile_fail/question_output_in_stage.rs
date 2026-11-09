use kaalang::kaa;

#[kaa]
fn invalid(take: bool) {
    #[kaa::question("Enter the stage?")]
    let (yes, no) = |take| take;

    #[kaa::action("Enter the stage.")]
    let go = |yes| ();

    #[kaa::cycle("Stay on the other branch.")]
    |no| loop {
        continue;
    };

    #[kaa::stage("Use the unmerged question output.")]
    |go| {
        |yes| return;
    };
}

fn main() {}
