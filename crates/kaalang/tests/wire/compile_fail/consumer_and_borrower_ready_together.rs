use kaalang::kaalang;

#[kaalang]
fn invalid(input: u32) -> (u32, u32) {
    #[action("Borrow the input.")]
    |&input| -> first { *input };

    #[action("Consume the input while the borrower is ready.")]
    |input| -> second { input };

    #[action("Pair the two values.")]
    |first, second| -> result { (first, second) };
}

fn main() {}
