use kaalang::kaalang;

#[kaalang]
fn invalid(input: u32) {
    #[action("Produce an unused value.")]
    |input| -> unused { input };

    #[end]
    || {};
}

fn main() {}
