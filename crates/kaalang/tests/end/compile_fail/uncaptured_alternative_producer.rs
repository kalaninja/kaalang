use kaalang::kaalang;

#[kaalang]
fn invalid(condition: bool) -> u32 {
    #[question("Choose a path.")]
    |condition| -> (yes, no) { condition };

    #[action("Build the value, a stray wire, and the settled marker.")]
    |yes| -> (value, stray, settled) { (1, (), ()) };

    #[action("Build the other value, its stray wire, and a tail.")]
    |no| -> (value, stray, tail) { (2, (), ()) };

    #[action("Consume the stray wire only where the no branch is selected.")]
    |stray, tail| -> settled { drop((stray, tail)) };

    #[action("Use the value once both branches have settled.")]
    |value, settled| -> result { value };
}

fn main() {}
