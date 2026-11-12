use kaalang::kaa;

#[kaa]
fn invalid(input: u32) -> u32 {
    #[kaa::question("Is the input nonzero?")]
    let (yes, no) = |&input| { *input };

    #[kaa::action("Produce the yes result.")]
    let end = |yes, &input| { *input };

    #[kaa::action("Produce the no result.")]
    let end = |no, &input| { *input };

    |end| return end;
}

fn main() {}
