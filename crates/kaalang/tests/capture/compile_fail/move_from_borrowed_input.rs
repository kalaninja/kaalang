use kaalang::kaalang;

#[kaalang]
fn invalid(input: String) -> usize {
    #[action("Move out of a borrowed input.")]
    let result = |&input| { input.into_bytes().len() };
}

fn main() {}
