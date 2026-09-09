use kaalang::kaalang;

#[kaalang]
fn invalid(input: u32) -> u32 {
    #[question("Ask a question with one output.")]
    let yes = |&input| { true };
}

fn main() {}
