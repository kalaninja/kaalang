use kaalang::kaalang;

#[kaalang]
fn invalid(input: u32) -> String {
    #[action("Copy the number.")]
    |input| -> result { input };

    #[end]
    |result| {};
}

fn main() {}
