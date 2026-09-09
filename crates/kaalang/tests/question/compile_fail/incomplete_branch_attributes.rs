use kaalang::kaalang;

#[kaalang]
fn invalid(condition: bool) -> u8 {
    #[question("Choose a branch.")]
    #[yes]
    |condition| -> (yes, no) { condition };
}

fn main() {}
