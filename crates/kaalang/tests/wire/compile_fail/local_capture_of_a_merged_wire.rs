use kaalang::kaalang;

// The shared name denotes the merged value. A branch-local capture needs a
// separate name; its gate cannot select one occurrence of the merged wire.
#[kaalang]
fn invalid(condition: bool) -> u8 {
    #[question("Choose a value.")]
    let (yes, no) = |condition| { condition };

    #[action("Build the yes value and local gate.")]
    let (value, yes_gate) = |yes| { (1u8, ()) };

    #[action("Build the no value and local gate.")]
    let (value, no_gate) = |no| { (2u8, ()) };

    #[action("Use the value inside the yes branch.")]
    let result = |value, yes_gate| { value };

    #[action("Use the value inside the no branch.")]
    let result = |value, no_gate| { value };
}

fn main() {}
