use kaalang::kaalang;

#[kaalang]
fn invalid(condition: bool) -> u8 {
    #[question("Choose a branch.")]
    #[yes("")]
    #[no]
    |condition| -> (yes, no) { condition };
}

fn main() {}
