use kaalang::kaalang;

#[kaalang]
fn invalid(input: u32) -> u32 {
    #[action("Use bracketed inputs.")]
    let output = |[input]| { input };
}

fn main() {}
