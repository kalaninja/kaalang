use kaalang::kaalang;

#[kaalang]
fn invalid(condition: bool) -> u32 {
    #[question("Which way?")]
    let (yes, no) = |condition| { condition };

    #[action("Build the yes value and a marker.")]
    let (selected, plain) = |yes| { (1, ()) };

    #[action("Build the no value and a note.")]
    let (selected, extra) = |no| { (2, 7) };

    #[action("Take the shared step.")]
    let stepped = |selected| { selected * 10 };

    #[action("Use the note after the shared step.")]
    let end = |stepped, extra| { stepped + extra };

    #[action("Finish without the note.")]
    let end = |stepped, plain| { stepped };

    |end| return end;
}

fn main() {}
