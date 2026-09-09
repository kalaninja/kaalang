use kaalang::kaalang;

#[kaalang]
fn invalid(input: u32) -> (u32, u32) {
    #[action("Produce two outputs with one name.")]
    let (result, result) = |input| { (input, input) };
}

fn main() {}
