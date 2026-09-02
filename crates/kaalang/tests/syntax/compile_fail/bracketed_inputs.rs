use kaalang::kaalang;

#[kaalang]
fn invalid(input: u32) -> u32 {
    #[action("Use bracketed inputs.")]
    |[input]| -> output { input };
}

fn main() {}
