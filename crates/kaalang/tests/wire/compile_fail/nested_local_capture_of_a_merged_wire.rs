use kaalang::kaalang;

// The nested branches and the direct branch all produce the shared name, so
// every capture takes the merged value. A gate cannot single out one of them.
#[kaalang]
fn invalid(outer: bool, late: bool) -> u8 {
    #[question("Take the nested part?")]
    |outer| -> (nested, direct) { outer };

    #[question("Converge late?")]
    |nested, late| -> (early, delayed) { late };

    #[action("Build the shared value early.")]
    |early| -> (shared, early_gate) { (1u8, ()) };

    #[action("Build the shared value late.")]
    |delayed| -> (shared, late_gate) { (2u8, ()) };

    #[action("Build the shared value directly.")]
    |direct| -> (shared, direct_gate) { (3u8, ()) };

    #[action("Use the shared value inside the early branch.")]
    |shared, early_gate| -> result { shared };

    #[action("Use the shared value inside the late branch.")]
    |shared, late_gate| -> result { shared };

    #[action("Use the shared value in the direct branch.")]
    |shared, direct_gate| -> result { shared };
}

fn main() {}
