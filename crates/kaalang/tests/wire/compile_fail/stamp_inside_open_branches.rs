use kaalang::kaalang;

// An action with no inputs follows the same rule as one with only common
// inputs: capturing nothing from a branch does not place it in one, and its
// consumers do not place it there either.
#[kaalang]
fn invalid(flag: bool) -> u64 {
    #[question("Which way?")]
    let (yes, no) = |flag| { flag };

    #[action("Stamp.")]
    let stamp = || { 7u64 };

    #[action("Finish yes.")]
    let result = |yes, stamp| { stamp };

    #[action("Finish no.")]
    let result = |no, stamp| { stamp + 1 };
}

fn main() {}
