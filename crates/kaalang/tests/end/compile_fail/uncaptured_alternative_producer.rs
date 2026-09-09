use kaalang::kaalang;

#[kaalang]
fn invalid(condition: bool) -> u32 {
    #[question("Choose a branch.")]
    let (yes, no) = |condition| { condition };

    #[action("Build the value, a stray wire, and the settled marker.")]
    let (value, stray, settled) = |yes| { (1, (), ()) };

    #[action("Build the other value, its stray wire, and a tail.")]
    let (value, stray, tail) = |no| { (2, (), ()) };

    #[action("Consume the stray wire only where the no branch is selected.")]
    let settled = |stray, tail| { drop((stray, tail)) };

    #[action("Use the value once both branches have settled.")]
    let result = |value, settled| { value };
}

fn main() {}
