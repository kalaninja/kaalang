use kaalang::kaalang;

#[kaalang]
fn invalid(input: u32) -> u32 {
    #[action(Copy the input.)]
    let output = |input| { input };
}

fn main() {}
