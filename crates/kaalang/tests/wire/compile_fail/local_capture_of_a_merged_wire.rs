use kaalang::kaalang;

// The shared name denotes the merged value. A branch-local capture needs a
// separate name; its gate cannot select one occurrence of the merged wire.
#[kaalang]
fn invalid(condition: bool) -> u8 {
    #[question("Choose a value.")]
    |condition| -> (yes, no) { condition };

    #[action("Build the yes value and local gate.")]
    |yes| -> (value, yes_gate) { (1u8, ()) };

    #[action("Build the no value and local gate.")]
    |no| -> (value, no_gate) { (2u8, ()) };

    #[action("Use the value inside the yes branch.")]
    |value, yes_gate| -> result { value };

    #[action("Use the value inside the no branch.")]
    |value, no_gate| -> result { value };
}

fn main() {}
