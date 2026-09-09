use kaalang::kaalang;

#[kaalang]
fn invalid(input: u32) -> u32 {
    #[action("Produce the result.")]
    let result = |input| { input };

    #[end]
    |result| {};
}

fn main() {}
