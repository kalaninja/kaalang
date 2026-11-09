use kaalang::kaa;

#[kaa]
fn invalid(input: u32) -> u32 {
    #[kaa::question("Ask a question with one output.")]
    let yes = |&input| { true };
}

fn main() {}
