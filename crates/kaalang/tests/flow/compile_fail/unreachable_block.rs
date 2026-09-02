use kaalang::kaalang;

#[kaalang]
fn invalid(input: u32, extra: u32) -> u32 {
    #[action("Consume the extra input.")]
    |&input, extra| -> combined { *input + extra };

    #[action("Produce the result.")]
    |&combined| -> finished { *combined };

    #[action("Reuse the already consumed input.")]
    |combined, extra| -> _reused { combined + extra };

    #[end]
    |finished| {};
}

fn main() {}
