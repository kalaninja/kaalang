use kaalang::kaalang;

#[kaalang]
fn invalid(input: u32) -> u32 {
    #[question("Use no inputs.")]
    let (yes, no) = || { true };
}

fn main() {}
