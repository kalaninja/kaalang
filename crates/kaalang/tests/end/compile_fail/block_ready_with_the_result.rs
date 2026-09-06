use kaalang::kaalang;

#[kaalang]
fn invalid(input: u32, other: u32) -> u32 {
    #[action("Produce the result.")]
    |input| -> result { input };

    #[action("Work that never reaches the result.")]
    |other| -> _stray { other };
}

fn main() {}
