use kaalang::kaa;

#[kaa]
fn invalid(input: u32) {
    #[kaa::action("Produce a used and an unused value.")]
    let (used, unused) = |input| (input, input);

    |used| return used;
}

fn main() {}
