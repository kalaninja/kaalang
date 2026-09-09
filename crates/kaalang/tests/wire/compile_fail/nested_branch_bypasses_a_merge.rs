use kaalang::kaalang;

#[kaalang]
fn invalid(outer: bool, inner: bool) -> u32 {
    #[question("Take the nested branch?")]
    let (nested, direct) = |outer| { outer };

    #[question("Join the shared step?")]
    let (skip, join) = |nested, inner| { inner };

    #[action("Build the refined value.")]
    let shared = |join| { 1u32 };

    #[action("Build the direct value.")]
    let shared = |direct| { 2u32 };

    #[action("Add ten in the shared step.")]
    let ready = |shared| { shared + 10 };

    #[action("Bypass the shared step and rejoin at the later merge.")]
    let ready = |skip| { 100u32 };

    #[action("Use the later merge.")]
    let result = |ready| { ready + 1 };
}

fn main() {}
