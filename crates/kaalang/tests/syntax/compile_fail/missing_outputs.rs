use kaalang::kaalang;

// An expression statement declares no outputs, and a question needs two.
#[kaalang]
fn invalid(input: u32) -> u32 {
    #[question("Omit the question outputs.")]
    |&input| true;

    |input| return input;
}

fn main() {}
