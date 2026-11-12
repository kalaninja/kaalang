use kaalang::kaa;

// `let ()` declares no wires, and a question needs two.
#[kaa]
fn invalid(input: u32) -> u32 {
    #[kaa::question("Ask without outputs.")]
    let () = |&input| { *input > 0 };

    |input| return input;
}

fn main() {}
