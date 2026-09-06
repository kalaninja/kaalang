use kaalang::kaalang;

#[kaalang]
fn invalid(input: u32) -> u32 {
    #[action("Produce the result.")]
    |input| -> result { input };

    #[action("Capture the wire that finishes the flow.")]
    |result| -> doubled { result * 2 };
}

fn main() {}
