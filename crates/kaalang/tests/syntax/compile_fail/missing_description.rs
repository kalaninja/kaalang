#[kaalang::kaalang]
fn invalid(input: u32) -> u32 {
    // A comment is deliberately not the block description.
    #[action]
    let output = |input| { input };
}

fn main() {}
