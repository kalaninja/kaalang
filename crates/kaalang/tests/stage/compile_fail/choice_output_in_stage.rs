use kaalang::kaa;

#[kaa]
fn invalid(take: bool) -> String {
    #[kaa::choice("Enter the stage?")]
    #[kaa::case("Provide an owner.")]
    #[kaa::case("Stay here.")]
    let (owner, stay) = |take| match take {
        true => String::from("owned"),
        false => (),
    };

    #[kaa::action("Enter the stage.")]
    let go = |&owner| ();

    #[kaa::cycle("Stay on the other branch.")]
    |stay| loop {
        continue;
    };

    #[kaa::stage("Use the unmerged choice output.")]
    |go| {
        |owner| return owner;
    };
}

fn main() {}
