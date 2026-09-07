use kaalang::kaalang;

#[kaalang]
fn invalid(outer: bool, value: u8, inner: bool) -> u8 {
    #[question("Take the choice?")]
    |outer, &value, &inner| -> (yes, no) { outer };

    #[choice("Which case?")]
    #[case("Refine further.")]
    #[case("Build the shared value directly.")]
    |yes, value| -> (refine, direct) {
        match value {
            0 => (),
            _ => (),
        }
    };

    #[question("Join the shared step?")]
    |refine, inner| -> (join, skip) { inner };

    #[action("Build the shared value on the joining branch.")]
    |join| -> shared { 1 };

    #[action("Build the shared value on the direct case.")]
    |direct| -> shared { 2 };

    #[action("Take the shared step of both cases.")]
    |shared| -> ready { shared + 10 };

    #[action("Skip the shared step and go straight to the final step.")]
    |skip| -> ready { 100 };

    #[action("Build the ready value on the no branch.")]
    |no| -> ready { 200 };

    #[action("Take the final step every branch shares.")]
    |ready| -> result { ready + 1 };
}

fn main() {}
