use kaalang::kaa;

// The shared name denotes the merged value. A branch-local capture needs a
// separate name; its gate cannot select one occurrence of the merged wire.
#[kaa]
fn invalid(condition: bool) -> u8 {
    #[kaa::question("Choose a value.")]
    let (yes, no) = |condition| { condition };

    #[kaa::action("Build the yes value and local gate.")]
    let (value, yes_gate) = |yes| { (1, ()) };

    #[kaa::action("Build the no value and local gate.")]
    let (value, no_gate) = |no| { (2, ()) };

    #[kaa::action("Use the value inside the yes branch.")]
    let end = |value, yes_gate| { value };

    #[kaa::action("Use the value inside the no branch.")]
    let end = |value, no_gate| { value };

    |end| return end;
}

fn main() {}
