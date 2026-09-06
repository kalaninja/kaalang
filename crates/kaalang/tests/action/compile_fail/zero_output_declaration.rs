use kaalang::kaalang;

#[kaalang]
fn invalid(input: u32) {
    #[action("Declare no outputs.")]
    |input| -> () { input + 1 };
}

fn main() {}
