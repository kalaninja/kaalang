use kaalang::kaalang;

#[kaalang]
fn invalid(input: u32) -> u32 {
    #[question("Is the input nonzero?")]
    |&input| -> (yes, no) { *input };

    #[action("Produce the yes result.")]
    |yes, &input| -> result { *input };

    #[action("Produce the no result.")]
    |no, &input| -> result { *input };
}

fn main() {}
