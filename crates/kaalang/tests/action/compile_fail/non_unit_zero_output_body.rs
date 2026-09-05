use kaalang::kaalang;

#[kaalang]
fn invalid(input: u32) {
    #[action("Produce a value while declaring no outputs.")]
    |input| -> () { input + 1 };

    #[end]
    || {};
}

fn main() {}
