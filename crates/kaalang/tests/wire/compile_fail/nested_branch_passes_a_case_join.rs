use kaalang::kaalang;

#[kaalang]
fn invalid(outer: bool, value: u8, inner: bool) -> u8 {
    #[question("Take the choice?")]
    let (yes, no) = |outer, &value, &inner| { outer };

    #[choice("Which case?")]
    #[case("Refine further.")]
    #[case("Build the shared value directly.")]
    let (refine, direct) = |yes, value| {
        match value {
            0 => (),
            _ => (),
        }
    };

    #[question("Join the shared step?")]
    let (join, skip) = |refine, inner| { inner };

    #[action("Build the shared value on the joining branch.")]
    let shared = |join| { 1 };

    #[action("Build the shared value on the direct case.")]
    let shared = |direct| { 2 };

    #[action("Take the shared step of both cases.")]
    let ready = |shared| { shared + 10 };

    #[action("Skip the shared step and go straight to the final step.")]
    let ready = |skip| { 100 };

    #[action("Build the ready value on the no branch.")]
    let ready = |no| { 200 };

    #[action("Take the final step every branch shares.")]
    let end = |ready| { ready + 1 };
}

fn main() {}
