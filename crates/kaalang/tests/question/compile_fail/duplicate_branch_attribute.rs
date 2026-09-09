use kaalang::kaalang;

#[kaalang]
fn invalid(condition: bool) -> u8 {
    #[question("Choose a branch.")]
    #[yes]
    #[yes]
    |condition| -> (first, second) { condition };
}

fn main() {}
