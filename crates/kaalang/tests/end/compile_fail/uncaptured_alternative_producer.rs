use kaalang::kaalang;

#[kaalang]
fn invalid(condition: bool) -> u32 {
    #[question("Choose a path.")]
    |condition| -> (yes, no) { condition };

    #[action("Build the result and a stray wire.")]
    |yes| -> (result, stray) { (1, ()) };

    #[action("Build the other result and its stray wire.")]
    |no| -> (result, stray, tail) { (2, (), ()) };

    #[action("Consume the stray wire only where the no branch is selected.")]
    |stray, tail| -> () { drop((stray, tail)) };

    #[end]
    |result| {};
}

fn main() {}
