use kaalang::kaa;

#[kaa]
fn invalid(input: u32) -> u32 {
    #[kaa::action("Use bracketed inputs.")]
    let output = |[input]| { input };
}

fn main() {}
