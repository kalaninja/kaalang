use kaalang::kaalang;

// Every execution finishes with `result`, and each branch leaves its own
// marker behind. No execution provides both, so the block needing both never
// runs.
#[kaalang]
fn invalid(condition: bool) -> u32 {
    #[question("Which way?")]
    |condition| -> (yes, no) { condition };

    #[action("Finish the yes branch and mark it.")]
    |yes| -> (_left, result) { ((), 1) };

    #[action("Finish the no branch and mark it.")]
    |no| -> (_right, result) { ((), 2) };

    #[action("Combine two markers no execution provides together.")]
    |_left, _right| -> _reused { () };
}

fn main() {}
