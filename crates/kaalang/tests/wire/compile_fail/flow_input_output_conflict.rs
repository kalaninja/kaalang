use kaalang::kaa;

#[kaa]
fn invalid(input: u32) -> u32 {
    #[kaa::action("Reuse the flow input name as an output.")]
    let input = |&input| { *input };
}

fn main() {}
