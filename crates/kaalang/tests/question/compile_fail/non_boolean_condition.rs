use kaalang::kaalang;

#[kaalang]
fn invalid(input: u32) -> u32 {
    #[question("Is the input nonzero?")]
    let (yes, no) = |&input| { *input };

    #[action("Produce the yes result.")]
    let end = |yes, &input| { *input };

    #[action("Produce the no result.")]
    let end = |no, &input| { *input };
}

fn main() {}
