use kaalang::kaa;

#[kaa]
fn invalid(input: String) -> usize {
    #[kaa::action("Move out of a borrowed input.")]
    let end = |&input| { input.into_bytes().len() };

    |end| return end;
}

fn main() {}
