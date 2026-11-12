use kaalang::kaa;

#[kaa]
fn invalid(refine: bool, value: u8) -> u32 {
    #[kaa::question("Refine the value?")]
    let (nested, direct) = |refine| { refine };

    #[kaa::choice("Join the shared step?")]
    #[kaa::case("Join.")]
    #[kaa::case("Skip.")]
    let (join, skip) = |nested, value| {
        match value {
            0 => (),
            _ => (),
        }
    };

    #[kaa::action("Build the refined value.")]
    let shared = |join| { 1 };

    #[kaa::action("Build the direct value.")]
    let shared = |direct| { 2 };

    #[kaa::action("Add ten in the shared step.")]
    let ready = |shared| { shared + 10 };

    #[kaa::action("Bypass the shared step.")]
    let ready = |skip| { 100 };

    #[kaa::action("Use the later merge.")]
    let end = |ready| { ready + 1 };

    |end| return end;
}

fn main() {}
