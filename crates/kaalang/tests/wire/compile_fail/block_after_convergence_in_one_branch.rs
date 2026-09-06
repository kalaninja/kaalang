use kaalang::kaalang;

#[kaalang]
fn invalid(condition: bool) -> u32 {
    #[question("Which way?")]
    |condition| -> (yes, no) { condition };

    #[action("Build the yes value and a marker.")]
    |yes| -> (selected, plain) { (1, ()) };

    #[action("Build the no value and a note.")]
    |no| -> (selected, extra) { (2, 7u32) };

    #[action("Take the shared step.")]
    |selected| -> stepped { selected * 10 };

    #[action("Use the note after the shared step.")]
    |stepped, extra| -> result { stepped + extra };

    #[action("Finish without the note.")]
    |stepped, plain| -> result { stepped };
}

fn main() {}
