use kaalang::kaa;

#[kaa]
fn invalid(take: bool, first: bool) -> u8 {
    #[kaa::question("Enter the stage?")]
    let (yes, no) = |take| take;

    #[kaa::question("Prepare the first value?")]
    let (left, right) = |yes, first| first;

    #[kaa::action("Prepare the first value.")]
    let shared = |left| 1;

    #[kaa::action("Prepare the second value.")]
    let shared = |right| 2;

    #[kaa::action("Enter the stage from the outer branch.")]
    let go = |shared| ();

    #[kaa::cycle("Stay on the other branch.")]
    |no| loop {
        continue;
    };

    #[kaa::stage("Use a merge still inside a branch.")]
    |go| {
        |shared| return shared;
    };
}

fn main() {}
