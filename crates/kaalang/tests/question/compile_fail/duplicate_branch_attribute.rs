use kaalang::kaa;

#[kaa]
fn invalid(condition: bool) -> u8 {
    #[kaa::question("Choose a branch.")]
    #[kaa::yes]
    #[kaa::yes]
    let (first, second) = |condition| { condition };
}

fn main() {}
