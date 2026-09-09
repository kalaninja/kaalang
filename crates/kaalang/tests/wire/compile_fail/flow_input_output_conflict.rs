use kaalang::kaalang;

#[kaalang]
fn invalid(input: u32) -> u32 {
    #[action("Reuse the flow input name as an output.")]
    let input = |&input| { *input };
}

fn main() {}
