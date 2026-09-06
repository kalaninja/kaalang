use kaalang::kaalang;

#[kaalang]
fn invalid(input: u32) {
    #[action("Produce a used and an unused value.")]
    |input| -> (used, unused) { (input, input) };

    #[action("Finish with the used value.")]
    |used| -> result { used };
}

fn main() {}
