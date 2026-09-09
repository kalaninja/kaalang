use kaalang::kaalang;

// Omitting the outputs leaves the body unbraced, which is reported first.
#[kaalang]
fn invalid(input: u32) -> u32 {
    #[question("Omit the question outputs.")]
    |&input| true;

    #[action("Finish.")]
    |input| -> result { input };
}

fn main() {}
