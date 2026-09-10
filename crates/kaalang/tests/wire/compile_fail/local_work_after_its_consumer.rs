use kaalang::kaalang;

// The yes branch still owes `noted` to the `selected` merge, so recording it
// below the block that captures the merged value leaves it nowhere to run.
#[kaalang]
fn invalid(condition: bool) -> u32 {
    #[question("Choose a value.")]
    let (yes, no) = |condition| { condition };

    #[action("Build the yes value and local note.")]
    let (selected, local_note) = |yes| { (21, 1u8) };

    #[action("Build the no value, which has no note to record.")]
    let (selected, noted) = |no| { (34, ()) };

    #[action("Use the merged value.")]
    let used = |selected| { selected * 2 };

    #[action("Record the note after leaving the yes branch.")]
    let noted = |local_note| { let _ = local_note; };

    #[action("Finish.")]
    let end = |used, noted| { used };
}

fn main() {}
