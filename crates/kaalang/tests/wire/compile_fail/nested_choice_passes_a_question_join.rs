use kaalang::kaalang;

#[kaalang]
fn invalid(refine: bool, value: u8) -> u32 {
    #[question("Refine the value?")]
    |refine| -> (nested, direct) { refine };

    #[choice("Join the shared step?")]
    #[case("Join.")]
    #[case("Skip.")]
    |nested, value| -> (join, skip) {
        match value {
            0 => (),
            _ => (),
        }
    };

    #[action("Build the refined value.")]
    |join| -> shared { 1 };

    #[action("Build the direct value.")]
    |direct| -> shared { 2 };

    #[action("Add ten in the shared step.")]
    |shared| -> ready { shared + 10 };

    #[action("Bypass the shared step.")]
    |skip| -> ready { 100 };

    #[action("Use the later merge.")]
    |ready| -> result { ready + 1 };
}

fn main() {}
