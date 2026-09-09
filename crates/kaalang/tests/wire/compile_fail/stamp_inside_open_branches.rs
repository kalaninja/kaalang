use kaalang::kaalang;

// An action with no inputs follows the same rule as one with only common
// inputs: capturing nothing from a branch does not place it in one, and its
// consumers do not place it there either.
#[kaalang]
fn invalid(flag: bool) -> u64 {
    #[question("Which way?")]
    |flag| -> (yes, no) { flag };

    #[action("Stamp.")]
    || -> stamp { 7u64 };

    #[action("Finish yes.")]
    |yes, stamp| -> result { stamp };

    #[action("Finish no.")]
    |no, stamp| -> result { stamp + 1 };
}

fn main() {}
