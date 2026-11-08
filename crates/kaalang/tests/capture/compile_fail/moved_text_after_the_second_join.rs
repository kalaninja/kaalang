use kaalang::kaa;

// The merged text moves into its length, then a common action after the
// second join takes it again. Rust reports the use after the move.
#[kaa]
fn invalid(condition: bool, long: bool) -> usize {
    #[kaa::question("Which text?")]
    let (first, second) = |condition| { condition };

    #[kaa::action("Build the first text.")]
    let text = |first| { String::from("abc") };

    #[kaa::action("Build the second text.")]
    let text = |second| { String::from("de") };

    #[kaa::action("Measure the merged text.")]
    let length = |text| { text.len() };

    #[kaa::question("Is it long enough?")]
    let (yes, no) = |&length, long| { long && *length > 2 };

    #[kaa::action("Keep the length.")]
    let selected = |yes, length| { length };

    #[kaa::action("Report nothing.")]
    let selected = |no, length| { length };

    #[kaa::action("Use the moved text after the join.")]
    let end = |selected, text| { selected + text.len() };

    |end| return end;
}

fn main() {}
