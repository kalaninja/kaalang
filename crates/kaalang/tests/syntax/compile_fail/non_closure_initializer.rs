use kaalang::kaalang;

#[kaalang]
fn invalid(input: u32) -> u32 {
    #[action("Copy the input without a closure-shaped block.")]
    let end = input;
}

fn main() {}
