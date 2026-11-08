use kaalang::kaa;

#[kaa]
fn singleton_output_body_type(input: u32) -> u32 {
    #[kaa::action("Expect a singleton tuple but produce a scalar.")]
    let (end,) = |input| { input };

    |end| return end;
}

fn main() {}
