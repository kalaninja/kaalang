use kaalang::kaa;

#[kaa]
fn invalid(condition: bool) -> u32 {
    #[kaa::question("Which way?")]
    let (yes, no) = |condition| { condition };

    #[kaa::action("Build the yes value and a marker.")]
    let (selected, plain) = |yes| { (1, ()) };

    #[kaa::action("Build the no value and a note.")]
    let (selected, extra) = |no| { (2, 7) };

    #[kaa::action("Take the shared step.")]
    let stepped = |selected| { selected * 10 };

    #[kaa::action("Use the note after the shared step.")]
    let end = |stepped, extra| { stepped + extra };

    #[kaa::action("Finish without the note.")]
    let end = |stepped, plain| { stepped };

    |end| return end;
}

fn main() {}
