use kaalang::kaalang;

// The block declares no outputs, so Rust checks its body against `()`.
#[kaalang]
fn invalid(input: u32) {
    #[action("Declare no outputs.")]
    |input| -> () { input + 1 };

    #[action("Finish.")]
    || -> result {};
}

fn main() {}
