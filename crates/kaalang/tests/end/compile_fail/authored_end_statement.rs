use kaalang::kaalang;

#[kaalang]
fn invalid(input: u32) -> u32 {
    #[action("Produce the result.")]
    let end = |input| { input };

    #[end]
    |end| {};
}

fn main() {}
