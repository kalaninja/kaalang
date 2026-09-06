use kaalang::kaalang;

// Each value must leave its own branch before the other branch can use it,
// but both branches still have local work waiting for the other merged value.
#[kaalang]
fn invalid(left: bool, right: bool) {
    #[question("Left enabled?")]
    |left| -> (left_yes, left_no) { left };

    #[question("Right enabled?")]
    |right| -> (right_yes, right_no) { right };

    #[action("Build the left value and local continuation.")]
    |left_yes| -> (left_value, left_continue) { (1u8, ()) };

    #[action("Build the other left value.")]
    |left_no| -> left_value { 0u8 };

    #[action("Build the right value and local continuation.")]
    |right_yes| -> (right_value, right_continue) { (1u8, ()) };

    #[action("Build the other right value.")]
    |right_no| -> right_value { 0u8 };

    #[action("Use the right value inside the left branch.")]
    |left_continue, &right_value| -> () {};

    #[action("Use the left value inside the right branch.")]
    |right_continue, &left_value| -> () {};

    #[end]
    || {};
}

fn main() {}
