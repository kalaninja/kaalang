use kaalang::kaalang;

#[kaalang]
fn invalid(input: String) -> usize {
    #[action("Move out of a borrowed input.")]
    let end = |&input| { input.into_bytes().len() };

    |end| return end;
}

fn main() {}
