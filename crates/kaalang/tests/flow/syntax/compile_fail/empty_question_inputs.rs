use kaalang::kaa;

#[kaa]
fn invalid(input: u32) -> u32 {
    #[kaa::question("Use no inputs.")]
    let (yes, no) = || { true };
}

fn main() {}
