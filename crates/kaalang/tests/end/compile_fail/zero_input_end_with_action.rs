use kaalang::kaalang;

#[kaalang]
fn invalid(input: u32) {
    #[action("Log the input.")]
    |input| -> _logged { () };

    #[end]
    || {};
}

fn main() {}
