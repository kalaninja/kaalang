use kaalang::kaalang;

// The flow breaks two rules at once. The second question opens its branches
// while the first question's are still separate, and the work needing both
// selected outputs leaves three of the four executions without an `end`
// wire. Branch placement is the diagnostic reported.
#[kaalang]
fn invalid(left: bool, right: bool) -> u8 {
    #[question("Enable the left input.")]
    let (a, _left_no) = |left| { left };

    #[question("Enable the right input.")]
    let (b, _right_no) = |right| { right };

    #[action("Require both selected outputs.")]
    let end = |a, b| { 1 };

    |end| return end;
}

fn main() {}
