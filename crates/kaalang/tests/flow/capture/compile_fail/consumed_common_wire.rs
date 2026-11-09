use kaalang::kaa;

// kaalang accepts the structure: the yes branch takes the common wire and the
// no branch only borrows it. Rust reports the use after that move.
#[kaa]
fn invalid(condition: bool, common: String) -> String {
    #[kaa::question("Choose a branch.")]
    let (yes, no) = |condition| { condition };

    #[kaa::action("Consume the common wire.")]
    let selected = |yes, common| { common };

    #[kaa::action("Preserve the common wire.")]
    let selected = |no, &common| { common.clone() };

    #[kaa::action("Use the selected and common wires.")]
    let end = |selected, common| { format!("{common}:{selected}") };

    |end| return end;
}

fn main() {}
