use kaalang::kaalang;

#[kaalang]
fn invalid(condition: bool) -> u32 {
    #[question("Which way?")]
    |condition| -> (yes, no) { condition };

    #[action("Build the yes value.")]
    |yes| -> selected { 1 };

    #[action("Build the no value and a note.")]
    |no| -> (selected, extra) { (2, 7u8) };

    #[action("Take the shared step.")]
    |selected| -> stepped { selected * 10 };

    #[action("Use the note after the shared step.")]
    |&stepped, extra| -> () { drop(extra) };

    #[end]
    |stepped| {};
}

fn main() {}
