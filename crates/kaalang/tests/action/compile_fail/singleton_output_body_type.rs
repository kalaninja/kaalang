use kaalang::kaalang;

#[kaalang]
fn singleton_output_body_type(input: u32) -> u32 {
    #[action("Expect a singleton tuple but produce a scalar.")]
    let (end,) = |input| { input };

    |end| return end;
}

fn main() {}
