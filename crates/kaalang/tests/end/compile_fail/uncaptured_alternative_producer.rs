use kaalang::kaalang;

#[kaalang]
fn invalid(condition: bool) -> u32 {
    #[question("Choose a path.")]
    |condition| -> (yes, no) { condition };

    #[action("Build the result and a stray wire.")]
    |yes| -> (result, stray) { (1, ()) };

    #[action("Build the other result and its stray wire.")]
    |&no| -> (result, stray) { (2, ()) };

    #[action("Consume the stray wire only where the no branch is selected.")]
    |stray, no| -> () { drop((stray, no)) };

    #[end]
    |result| {};
}

fn main() {}
