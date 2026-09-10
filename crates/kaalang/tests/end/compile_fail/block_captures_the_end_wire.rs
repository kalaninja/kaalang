use kaalang::kaalang;

#[kaalang]
fn invalid(input: u32) -> u32 {
    #[action("Produce the result.")]
    let end = |input| { input };

    #[action("Capture the wire that finishes the flow.")]
    let doubled = |end| { end * 2 };
}

fn main() {}
