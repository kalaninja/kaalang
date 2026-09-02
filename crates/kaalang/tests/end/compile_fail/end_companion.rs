use kaalang::kaalang;

#[kaalang]
fn invalid(input: u32) -> u32 {
    #[end]
    #[case("Not an End attribute.")]
    |input| {};
}

fn main() {}
