use kaalang::kaalang;

#[kaalang]
fn invalid(input: u32) -> u32 {
    #[action("Copy the input.")]
    #[case("The action has no cases.")]
    let output = |input| { input };
}

fn main() {}
