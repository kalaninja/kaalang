use kaalang::kaalang;

#[kaalang]
fn invalid(outer: bool, inner: bool) -> u32 {
    #[question("Take the nested branch?")]
    |outer| -> (nested, direct) { outer };

    #[question("Join the shared step?")]
    |nested, inner| -> (skip, join) { inner };

    #[action("Build the refined value.")]
    |join| -> shared { 1u32 };

    #[action("Build the direct value.")]
    |direct| -> shared { 2u32 };

    #[action("Add ten in the shared step.")]
    |shared| -> ready { shared + 10 };

    #[action("Bypass the shared step and rejoin at the later merge.")]
    |skip| -> ready { 100u32 };

    #[action("Use the later merge.")]
    |ready| -> result { ready + 1 };
}

fn main() {}
