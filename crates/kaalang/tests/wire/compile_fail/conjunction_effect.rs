use kaalang::kaalang;

// The flow breaks two rules at once. The second question opens its branches
// while the first question's are still separate, and the work needing both
// selected outputs leaves three of the four executions without a `result`
// wire. Branch placement is the diagnostic reported.
#[kaalang]
fn invalid(left: bool, right: bool) -> u8 {
    #[question("Enable the left input.")]
    |left| -> (a, _left_no) { left };

    #[question("Enable the right input.")]
    |right| -> (b, _right_no) { right };

    #[action("Require both selected outputs.")]
    |a, b| -> result { 1 };
}

fn main() {}
