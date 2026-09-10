use kaalang::kaalang;

// `let ()` declares no wires, and a question needs two.
#[kaalang]
fn invalid(input: u32) -> u32 {
    #[question("Ask without outputs.")]
    let () = |&input| { *input > 0 };

    #[action("Finish.")]
    let end = |input| { input };
}

fn main() {}
