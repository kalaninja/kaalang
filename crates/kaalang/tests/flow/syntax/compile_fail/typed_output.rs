use kaalang::kaa;

#[kaa]
fn typed_output(input: u32) -> u32 {
    #[kaa::action("Annotate the output type.")]
    let end: u32 = |input| { input };

    |end| return end;
}

fn main() {}
