use kaalang::kaalang;

// Forwarding each selected output through an action changes nothing: the work
// still runs only when both independent questions select yes.
#[kaalang]
fn invalid(left: bool, right: bool) {
    #[question("Left enabled?")]
    |left| -> (a, _left_no) { left };

    #[question("Right enabled?")]
    |right| -> (b, _right_no) { right };

    #[action("Accept the left signal.")]
    |a| -> a_ready {};

    #[action("Accept the right signal.")]
    |b| -> b_ready {};

    #[action("Do the work.")]
    |a_ready, b_ready| -> () { println!("work") };

    #[end]
    || {};
}

fn main() {}
