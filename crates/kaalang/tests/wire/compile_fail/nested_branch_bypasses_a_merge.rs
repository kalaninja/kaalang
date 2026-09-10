use kaalang::kaalang;

#[kaalang]
fn invalid(outer: bool, inner: bool) -> u32 {
    #[question("Take the nested branch?")]
    let (nested, direct) = |outer| { outer };

    #[question("Join the shared step?")]
    let (skip, join) = |nested, inner| { inner };

    #[action("Build the refined value.")]
    let shared = |join| { 1 };

    #[action("Build the direct value.")]
    let shared = |direct| { 2 };

    #[action("Add ten in the shared step.")]
    let ready = |shared| { shared + 10 };

    #[action("Bypass the shared step and rejoin at the later merge.")]
    let ready = |skip| { 100 };

    #[action("Use the later merge.")]
    let end = |ready| { ready + 1 };
}

fn main() {}
