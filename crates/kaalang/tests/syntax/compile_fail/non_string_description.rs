use kaalang::kaa;

#[kaa]
fn invalid(input: u32) -> u32 {
    #[kaa::action(Copy the input.)]
    let output = |input| { input };
}

fn main() {}
