use kaalang::kaa;

// Every execution finishes with `end`, and each branch leaves its own
// marker behind. No execution provides both, so the block needing both never
// runs.
#[kaa]
fn invalid(condition: bool) -> u32 {
    #[kaa::question("Which way?")]
    let (yes, no) = |condition| { condition };

    #[kaa::action("Finish the yes branch and mark it.")]
    let (_left, end) = |yes| { ((), 1) };

    #[kaa::action("Finish the no branch and mark it.")]
    let (_right, end) = |no| { ((), 2) };

    #[kaa::action("Combine two markers no execution provides together.")]
    let _reused = |_left, _right| { () };

    |end| return end;
}

fn main() {}
