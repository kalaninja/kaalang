use kaalang::kaalang;

#[kaalang]
fn invalid(input: u32) {
    #[action("Produce a used and an unused value.")]
    let (used, unused) = |input| (input, input);

    |used| return used;
}

fn main() {}
