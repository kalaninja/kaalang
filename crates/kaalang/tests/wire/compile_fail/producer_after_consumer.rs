use kaalang::kaalang;

#[kaalang]
fn invalid(condition: bool) -> u32 {
    #[question("Choose a value.")]
    |condition| -> (yes, no) { condition };

    #[action("Build the first value.")]
    |yes| -> selected { 1 };

    #[action("Use the selected value.")]
    |selected| -> result { selected };

    #[action("Build the later alternative.")]
    |no| -> selected { 2 };

    #[end]
    |result| {};
}

fn main() {}
