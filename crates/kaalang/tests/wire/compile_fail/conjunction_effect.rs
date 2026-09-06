use kaalang::kaalang;

#[kaalang]
fn invalid(left: bool, right: bool) {
    #[question("Enable the left input.")]
    |left| -> (a, _left_no) { left };

    #[question("Enable the right input.")]
    |right| -> (b, _right_no) { right };

    #[action("Require both selected outputs.")]
    |a, b| -> () {};

    #[end]
    || {};
}

fn main() {}
