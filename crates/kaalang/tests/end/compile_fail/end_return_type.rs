use kaalang::kaalang;

#[kaalang]
fn invalid(input: u32) -> String {
    #[action("Copy the number.")]
    let end = |input| { input };
}

fn main() {}
