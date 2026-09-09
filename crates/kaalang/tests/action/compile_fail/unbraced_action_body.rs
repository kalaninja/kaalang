use kaalang::kaalang;

// An omitted output arrow does not make the braces optional.
#[kaalang]
fn invalid(input: u32) -> u32 {
    #[action("Double the input without braces.")]
    |input| input * 2;

    #[action("Finish.")]
    || -> result { 0 };
}

fn main() {}
