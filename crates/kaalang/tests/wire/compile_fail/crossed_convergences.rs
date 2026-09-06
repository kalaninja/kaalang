use kaalang::kaalang;

// Each value must leave its own branch before the other branch can use it,
// but both branches still have local work waiting for the other merged value.
#[kaalang]
fn invalid(left: bool, right: bool) -> u8 {
    #[question("Left enabled?")]
    |left| -> (left_yes, left_no) { left };

    #[question("Right enabled?")]
    |right| -> (right_yes, right_no) { right };

    #[action("Build the left value and local continuation.")]
    |left_yes| -> (left_value, left_continue) { (1u8, ()) };

    #[action("Build the other left value.")]
    |left_no| -> (left_value, left_done) { (0u8, ()) };

    #[action("Build the right value and local continuation.")]
    |right_yes| -> (right_value, right_continue) { (1u8, ()) };

    #[action("Build the other right value.")]
    |right_no| -> (right_value, right_done) { (0u8, ()) };

    #[action("Use the right value inside the left branch.")]
    |left_continue, &right_value| -> left_done {};

    #[action("Use the left value inside the right branch.")]
    |right_continue, &left_value| -> right_done {};

    #[action("Finish once both branches are done.")]
    |left_done, right_done| -> result { 0u8 };
}

fn main() {}
