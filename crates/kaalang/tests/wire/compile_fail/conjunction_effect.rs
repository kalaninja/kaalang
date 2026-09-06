use kaalang::kaalang;

// The work needs both selected outputs, so three of the four executions leave
// the flow without its `result` wire.
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
