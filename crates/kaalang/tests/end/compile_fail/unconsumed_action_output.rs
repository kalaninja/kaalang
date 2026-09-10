use kaalang::kaalang;

#[kaalang]
fn invalid(input: u32) {
    #[action("Produce a used and an unused value.")]
    let (used, unused) = |input| { (input, input) };

    #[action("Finish with the used value.")]
    let end = |used| { used };
}

fn main() {}
