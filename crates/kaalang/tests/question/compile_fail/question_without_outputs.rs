use kaalang::kaalang;

// `-> ()` declares no wires, and a question needs two.
#[kaalang]
fn invalid(input: u32) -> u32 {
    #[question("Ask without outputs.")]
    |&input| -> () { *input > 0 };

    #[action("Finish.")]
    |input| -> result { input };
}

fn main() {}
