use kaalang::kaalang;

#[kaalang]
fn typed_output(input: u32) -> u32 {
    #[action("Annotate the output type.")]
    let result: u32 = |input| { input };
}

fn main() {}
