use kaalang::kaalang;

#[kaalang]
fn invalid(input: u32) -> u32 {
    #[end("Unexpected description.")]
    |input| {};
}

fn main() {}
