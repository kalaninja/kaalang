use kaalang::kaalang;

#[kaalang]
fn invalid(input: u32) -> u32 {
    #[end]
    |input| {};

    #[end]
    |input| {};
}

fn main() {}
