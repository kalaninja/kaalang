use kaalang::kaa;

#[kaa]
fn invalid(outer: bool, value: u8, inner: bool) -> u8 {
    #[kaa::question("Take the choice?")]
    let (yes, no) = |outer, &value, &inner| { outer };

    #[kaa::choice("Which case?")]
    #[kaa::case("Refine further.")]
    #[kaa::case("Build the shared value directly.")]
    let (refine, direct) = |yes, value| {
        match value {
            0 => (),
            _ => (),
        }
    };

    #[kaa::question("Join the shared step?")]
    let (join, skip) = |refine, inner| { inner };

    #[kaa::action("Build the shared value on the joining branch.")]
    let shared = |join| { 1 };

    #[kaa::action("Build the shared value on the direct case.")]
    let shared = |direct| { 2 };

    #[kaa::action("Take the shared step of both cases.")]
    let ready = |shared| { shared + 10 };

    #[kaa::action("Skip the shared step and go straight to the final step.")]
    let ready = |skip| { 100 };

    #[kaa::action("Build the ready value on the no branch.")]
    let ready = |no| { 200 };

    #[kaa::action("Take the final step every branch shares.")]
    let end = |ready| { ready + 1 };

    |end| return end;
}

fn main() {}
