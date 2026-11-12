use kaalang::kaa;

// An expression statement declares no outputs, and a question needs two.
#[kaa]
fn invalid(input: u32) -> u32 {
    #[kaa::question("Omit the question outputs.")]
    |&input| true;

    |input| return input;
}

fn main() {}
