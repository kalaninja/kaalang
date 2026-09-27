use kaalang::kaalang;

#[kaalang]
fn invalid(take: bool) -> String {
    #[question("Enter the stage?")]
    let (yes, no) = |take| take;

    #[action("Create a branch-local owner.")]
    let owner = |yes| String::from("owned");

    #[action("Enter the stage.")]
    let go = |&owner| ();

    #[cycle("Stay on the other branch.")]
    |no| {
        continue;
    };

    #[stage("Use the unmerged owner.")]
    |go| {
        |owner| return owner;
    };
}

fn main() {}
