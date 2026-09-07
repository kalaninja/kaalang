use kaalang::kaalang;

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
