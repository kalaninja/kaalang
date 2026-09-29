use kaalang::kaalang;

#[kaalang]
fn invalid(take: bool, first: bool) -> u8 {
    #[question("Enter the stage?")]
    let (yes, no) = |take| take;

    #[question("Prepare the first value?")]
    let (left, right) = |yes, first| first;

    #[action("Prepare the first value.")]
    let shared = |left| 1;

    #[action("Prepare the second value.")]
    let shared = |right| 2;

    #[action("Enter the stage from the outer branch.")]
    let go = |shared| ();

    #[cycle("Stay on the other branch.")]
    |no| {
        continue;
    };

    #[stage("Use a merge still inside a branch.")]
    |go| {
        |shared| return shared;
    };
}

fn main() {}
