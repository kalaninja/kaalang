use kaalang::kaalang;

#[kaalang]
fn invalid(take: bool) -> String {
    #[choice("Enter the stage?")]
    #[case("Provide an owner.")]
    #[case("Stay here.")]
    let (owner, stay) = |take| match take {
        true => String::from("owned"),
        false => (),
    };

    #[action("Enter the stage.")]
    let go = |&owner| ();

    #[cycle("Stay on the other branch.")]
    |stay| loop {
        continue;
    };

    #[stage("Use the unmerged choice output.")]
    |go| {
        |owner| return owner;
    };
}

fn main() {}
