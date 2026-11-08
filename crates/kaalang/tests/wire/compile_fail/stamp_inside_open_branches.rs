use kaalang::kaa;

// An action with no inputs follows the same rule as one with only common
// inputs: capturing nothing from a branch does not place it in one, and its
// consumers do not place it there either.
#[kaa]
fn invalid(flag: bool) -> u64 {
    #[kaa::question("Which way?")]
    let (yes, no) = |flag| { flag };

    #[kaa::action("Stamp.")]
    let stamp = || { 7 };

    #[kaa::action("Finish yes.")]
    let end = |yes, stamp| { stamp };

    #[kaa::action("Finish no.")]
    let end = |no, stamp| { stamp + 1 };

    |end| return end;
}

fn main() {}
