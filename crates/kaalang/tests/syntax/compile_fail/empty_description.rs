#[kaalang::kaa]
fn invalid(input: u32) -> u32 {
    #[kaa::action("")]
    let output = |input| { input };
}

fn main() {}
