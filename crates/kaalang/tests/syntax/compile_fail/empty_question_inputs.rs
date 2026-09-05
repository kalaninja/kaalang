use kaalang::kaalang;

#[kaalang]
fn invalid(input: u32) -> u32 {
    #[question("Use no inputs.")]
    || -> (yes, no) { true };
}

fn main() {}
