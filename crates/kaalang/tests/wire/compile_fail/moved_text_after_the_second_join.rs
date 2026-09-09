use kaalang::kaalang;

// The merged text moves into its length, then a common action after the
// second join takes it again. Rust reports the use after the move.
#[kaalang]
fn invalid(condition: bool, long: bool) -> usize {
    #[question("Which text?")]
    |condition| -> (first, second) { condition };

    #[action("Build the first text.")]
    |first| -> text { String::from("abc") };

    #[action("Build the second text.")]
    |second| -> text { String::from("de") };

    #[action("Measure the merged text.")]
    |text| -> length { text.len() };

    #[question("Is it long enough?")]
    |&length, long| -> (yes, no) { long && *length > 2 };

    #[action("Keep the length.")]
    |yes, length| -> selected { length };

    #[action("Report nothing.")]
    |no, length| -> selected { length };

    #[action("Use the moved text after the join.")]
    |selected, text| -> result { selected + text.len() };
}

fn main() {}
