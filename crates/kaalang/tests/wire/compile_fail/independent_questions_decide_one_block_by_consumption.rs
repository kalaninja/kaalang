use kaalang::kaalang;

// Every wire is produced in every execution, yet the no branches consume the
// tokens, so the work still runs only when both independent questions keep them.
#[kaalang]
fn invalid(left: bool, right: bool) {
    #[action("Create both tokens.")]
    || -> (x, y) { ((), ()) };

    #[question("Keep the left token?")]
    |left| -> (left_yes, left_no) { left };

    #[question("Keep the right token?")]
    |right| -> (right_yes, right_no) { right };

    #[action("Keep x.")]
    |left_yes| -> _left_done {};

    #[action("Consume x.")]
    |left_no, x| -> _left_done {};

    #[action("Keep y.")]
    |right_yes| -> _right_done {};

    #[action("Consume y.")]
    |right_no, y| -> _right_done {};

    #[action("Use both remaining tokens.")]
    |_left_done, _right_done, x, y| -> () { println!("work") };

    #[end]
    || {};
}

fn main() {}
