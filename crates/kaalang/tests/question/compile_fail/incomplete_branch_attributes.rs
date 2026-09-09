use kaalang::kaalang;

#[kaalang]
fn invalid(condition: bool) -> u8 {
    #[question("Choose a branch.")]
    #[yes]
    let (yes, no) = |condition| { condition };
}

fn main() {}
