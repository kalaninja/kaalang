use kaalang::kaa;

#[kaa]
fn invalid(condition: bool) -> u8 {
    #[kaa::question("Choose a branch.")]
    #[kaa::yes(description)]
    #[kaa::no]
    let (yes, no) = |condition| { condition };
}

fn main() {}
