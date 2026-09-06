use kaalang::kaalang;

#[kaalang]
fn invalid(outer: bool, late: bool) {
    #[question("Take the nested part?")]
    |outer| -> (nested, direct) { outer };

    #[question("Converge late?")]
    |nested, late| -> (early, delayed) { late };

    #[action("Build the shared value early.")]
    |early| -> shared { 1u8 };

    #[action("Build the stepped value and the note late.")]
    |delayed| -> (stepped, note) { (4u8, 40u8) };

    #[action("Build the shared value and the note directly.")]
    |direct| -> (shared, note) { (3u8, 30u8) };

    #[action("Take the shared step.")]
    |shared| -> stepped { shared };

    #[action("Use the note after the shared step.")]
    |stepped, note| -> () { drop((stepped, note)) };

    #[end]
    || {};
}

fn main() {}
