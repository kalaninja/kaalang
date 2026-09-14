use kaalang::kaalang;

#[kaalang]
fn invalid(refine: bool, value: u8) -> u32 {
    #[question("Refine the value?")]
    let (nested, direct) = |refine| { refine };

    #[choice("Join the shared step?")]
    #[case("Join.")]
    #[case("Skip.")]
    let (join, skip) = |nested, value| {
        match value {
            0 => (),
            _ => (),
        }
    };

    #[action("Build the refined value.")]
    let shared = |join| { 1 };

    #[action("Build the direct value.")]
    let shared = |direct| { 2 };

    #[action("Add ten in the shared step.")]
    let ready = |shared| { shared + 10 };

    #[action("Bypass the shared step.")]
    let ready = |skip| { 100 };

    #[action("Use the later merge.")]
    let end = |ready| { ready + 1 };

    |end| return end;
}

fn main() {}
