use kaalang::kaa;

// The nested branches and the direct branch all produce the shared name, so
// every capture takes the merged value. A gate cannot single out one of them.
#[kaa]
fn invalid(outer: bool, late: bool) -> u8 {
    #[kaa::question("Take the nested part?")]
    let (nested, direct) = |outer| { outer };

    #[kaa::question("Converge late?")]
    let (early, delayed) = |nested, late| { late };

    #[kaa::action("Build the shared value early.")]
    let (shared, early_gate) = |early| { (1, ()) };

    #[kaa::action("Build the shared value late.")]
    let (shared, late_gate) = |delayed| { (2, ()) };

    #[kaa::action("Build the shared value directly.")]
    let (shared, direct_gate) = |direct| { (3, ()) };

    #[kaa::action("Use the shared value inside the early branch.")]
    let end = |shared, early_gate| { shared };

    #[kaa::action("Use the shared value inside the late branch.")]
    let end = |shared, late_gate| { shared };

    #[kaa::action("Use the shared value in the direct branch.")]
    let end = |shared, direct_gate| { shared };

    |end| return end;
}

fn main() {}
