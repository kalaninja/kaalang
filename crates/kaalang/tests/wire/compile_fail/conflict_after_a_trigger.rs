use kaalang::kaalang;

// Running the first borrower first makes the consumer ready beside the second
// borrower. The flow is invalid even though the other order avoids the conflict.
#[kaalang]
fn invalid(x: u32) -> (u32, u32) {
    #[action("Borrow x and produce the trigger.")]
    |&x| -> trigger { *x };

    #[action("Borrow x independently.")]
    |&x| -> other { *x };

    #[action("Consume x once triggered.")]
    |trigger, x| -> result { trigger + x };

    #[end]
    |result, other| {};
}

fn main() {}
