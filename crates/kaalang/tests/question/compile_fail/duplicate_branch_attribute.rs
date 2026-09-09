use kaalang::kaalang;

#[kaalang]
fn invalid(condition: bool) -> u8 {
    #[question("Choose a branch.")]
    #[yes]
    #[yes]
    let (first, second) = |condition| { condition };
}

fn main() {}
