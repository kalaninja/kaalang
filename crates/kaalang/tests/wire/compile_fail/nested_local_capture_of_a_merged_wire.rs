use kaalang::kaalang;

// The nested branches and the direct branch all produce the shared name, so
// every capture takes the merged value. A gate cannot single out one of them.
#[kaalang]
fn invalid(outer: bool, late: bool) -> u8 {
    #[question("Take the nested part?")]
    let (nested, direct) = |outer| { outer };

    #[question("Converge late?")]
    let (early, delayed) = |nested, late| { late };

    #[action("Build the shared value early.")]
    let (shared, early_gate) = |early| { (1u8, ()) };

    #[action("Build the shared value late.")]
    let (shared, late_gate) = |delayed| { (2u8, ()) };

    #[action("Build the shared value directly.")]
    let (shared, direct_gate) = |direct| { (3u8, ()) };

    #[action("Use the shared value inside the early branch.")]
    let end = |shared, early_gate| { shared };

    #[action("Use the shared value inside the late branch.")]
    let end = |shared, late_gate| { shared };

    #[action("Use the shared value in the direct branch.")]
    let end = |shared, direct_gate| { shared };
}

fn main() {}
