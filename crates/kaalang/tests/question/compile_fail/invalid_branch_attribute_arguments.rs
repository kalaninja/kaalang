use kaalang::kaalang;

#[kaalang]
fn invalid(condition: bool) -> u8 {
    #[question("Choose a branch.")]
    #[yes(description)]
    #[no]
    let (yes, no) = |condition| { condition };
}

fn main() {}
