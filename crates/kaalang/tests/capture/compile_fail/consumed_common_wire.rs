use kaalang::kaalang;

// kaalang accepts the structure: the yes branch takes the common wire and the
// no branch only borrows it. Rust reports the use after that move.
#[kaalang]
fn invalid(condition: bool, common: String) -> String {
    #[question("Choose a branch.")]
    let (yes, no) = |condition| { condition };

    #[action("Consume the common wire.")]
    let selected = |yes, common| { common };

    #[action("Preserve the common wire.")]
    let selected = |no, &common| { common.clone() };

    #[action("Use the selected and common wires.")]
    let end = |selected, common| { format!("{common}:{selected}") };

    |end| return end;
}

fn main() {}
