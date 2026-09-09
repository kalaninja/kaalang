use kaalang::kaalang;

// The merged text moves into its length, then a common action after the
// second join takes it again. Rust reports the use after the move.
#[kaalang]
fn invalid(condition: bool, long: bool) -> usize {
    #[question("Which text?")]
    let (first, second) = |condition| { condition };

    #[action("Build the first text.")]
    let text = |first| { String::from("abc") };

    #[action("Build the second text.")]
    let text = |second| { String::from("de") };

    #[action("Measure the merged text.")]
    let length = |text| { text.len() };

    #[question("Is it long enough?")]
    let (yes, no) = |&length, long| { long && *length > 2 };

    #[action("Keep the length.")]
    let selected = |yes, length| { length };

    #[action("Report nothing.")]
    let selected = |no, length| { length };

    #[action("Use the moved text after the join.")]
    let result = |selected, text| { selected + text.len() };
}

fn main() {}
