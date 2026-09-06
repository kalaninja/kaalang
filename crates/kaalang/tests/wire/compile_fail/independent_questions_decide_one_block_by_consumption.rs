use kaalang::kaalang;

// Every wire is produced in every execution and every combination reaches the
// result, yet the block that uses both tokens still runs only when two
// independent questions both keep theirs.
#[kaalang]
fn invalid(left: bool, right: bool) -> u8 {
    #[action("Create both tokens.")]
    || -> (x, y) { (1u8, 2u8) };

    #[question("Keep the left token?")]
    |left| -> (left_yes, left_no) { left };

    #[question("Keep the right token?")]
    |right| -> (right_yes, right_no) { right };

    #[action("Keep x.")]
    |left_yes| -> left_kept {};

    #[action("Consume x.")]
    |left_no, x| -> left_dropped {};

    #[action("Keep y.")]
    |right_yes| -> right_kept {};

    #[action("Consume y.")]
    |right_no, y| -> right_dropped {};

    #[action("Use both remaining tokens.")]
    |left_kept, right_kept, x, y| -> result { x + y };

    #[action("Use the left token only.")]
    |left_kept, right_dropped, x| -> result { x };

    #[action("Use the right token only.")]
    |left_dropped, right_kept, y| -> result { y };

    #[action("Use neither token.")]
    |left_dropped, right_dropped| -> result { 0u8 };
}

fn main() {}
