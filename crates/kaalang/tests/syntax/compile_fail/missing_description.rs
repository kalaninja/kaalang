#[kaalang::kaa]
fn invalid(input: u32) -> u32 {
    // A comment is deliberately not the block description.
    #[kaa::action]
    let output = |input| { input };
}

fn main() {}
