use kaalang::kaa;

#[kaa]
fn invalid(take: bool) -> String {
    #[kaa::question("Enter the stage?")]
    let (yes, no) = |take| take;

    #[kaa::action("Create a branch-local owner.")]
    let owner = |yes| String::from("owned");

    #[kaa::action("Enter the stage.")]
    let go = |&owner| ();

    #[kaa::cycle("Stay on the other branch.")]
    |no| loop {
        continue;
    };

    #[kaa::stage("Use the unmerged owner.")]
    |go| {
        |owner| return owner;
    };
}

fn main() {}
