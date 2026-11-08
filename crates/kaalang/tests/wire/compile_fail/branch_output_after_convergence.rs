use kaalang::kaa;

// A branch output is an ordinary wire, but it stays branch-local: reading it
// again below its own merge is rejected like any other branch-local value, and
// not by a rule of its own.
#[kaa]
fn invalid(condition: bool) -> u32 {
    #[kaa::question("Which way?")]
    let (yes, no) = |condition| { condition };

    #[kaa::action("Build the yes value.")]
    let selected = |yes| { 1 };

    #[kaa::action("Build the no value and a note.")]
    let (selected, extra) = |no| { (2, 7) };

    #[kaa::action("Take the shared step.")]
    let stepped = |selected| { selected * 10 };

    #[kaa::action("Read the yes output again after the shared step.")]
    let end = |stepped, yes| { stepped };

    #[kaa::action("Finish with the note.")]
    let end = |stepped, extra| { stepped + extra };

    |end| return end;
}

fn main() {}
