use kaalang::kaalang;

#[kaalang]
fn invalid(condition: bool) -> u32 {
    #[question("Which way?")]
    let (yes, no) = |condition| { condition };

    #[action("Build the yes value.")]
    let selected = |yes| { 1 };

    #[action("Build the no value and a note.")]
    let (selected, extra) = |no| { (2, 7) };

    #[action("Take the shared step.")]
    let stepped = |selected| { selected * 10 };

    #[action("Read the yes output again after the shared step.")]
    let end = |stepped, yes| { stepped };

    #[action("Finish with the note.")]
    let end = |stepped, extra| { stepped + extra };

    |end| return end;
}

fn main() {}
