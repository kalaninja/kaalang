use kaalang::kaalang;

// kaalang accepts the structure: the yes branch takes the common wire and the
// no branch only borrows it. Rust reports the use after that move.
#[kaalang]
fn invalid(condition: bool, common: String) -> String {
    #[question("Choose a branch.")]
    |condition| -> (yes, no) { condition };

    #[action("Consume the common wire.")]
    |yes, common| -> selected { common };

    #[action("Preserve the common wire.")]
    |no, &common| -> selected { common.clone() };

    #[action("Use the selected and common wires.")]
    |selected, common| -> result { format!("{common}:{selected}") };
}

fn main() {}
