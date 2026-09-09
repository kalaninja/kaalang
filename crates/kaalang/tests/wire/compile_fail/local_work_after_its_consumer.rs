use kaalang::kaalang;

// The yes branch still owes `noted` to the `selected` merge, so recording it
// below the block that captures the merged value leaves it nowhere to run.
#[kaalang]
fn invalid(condition: bool) -> u32 {
    #[question("Choose a value.")]
    |condition| -> (yes, no) { condition };

    #[action("Build the yes value and local note.")]
    |yes| -> (selected, local_note) { (21, 1u8) };

    #[action("Build the no value, which has no note to record.")]
    |no| -> (selected, noted) { (34, ()) };

    #[action("Use the merged value.")]
    |selected| -> used { selected * 2 };

    #[action("Record the note after leaving the yes branch.")]
    |local_note| -> noted { let _ = local_note; };

    #[action("Finish.")]
    |used, noted| -> result { used };
}

fn main() {}
