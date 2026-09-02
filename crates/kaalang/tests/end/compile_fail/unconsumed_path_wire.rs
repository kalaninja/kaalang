use kaalang::kaalang;

#[kaalang]
fn invalid(condition: bool) -> u32 {
    #[question("Choose a path.")]
    |condition| -> (yes, no) { condition };

    #[action("Build a result and two stray wires.")]
    |yes| -> (result, first_stray, second_stray) { (1, (), ()) };

    #[action("Build the other result.")]
    |no| -> result { 2 };

    #[action("Declare an impossible first consumer.")]
    |first_stray, no| -> _first_ignored { () };

    #[action("Declare an impossible second consumer.")]
    |second_stray, no| -> _second_ignored { () };

    #[end]
    |result| {};
}

fn main() {}
