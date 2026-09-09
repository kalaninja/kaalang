use kaalang::kaalang;

#[kaalang]
fn invalid(input: String) -> usize {
    #[action("Move out of a borrowed input.")]
    |&input| -> result { input.into_bytes().len() };
}

fn main() {}
