use kaalang::kaalang;

#[kaalang]
fn invalid(input: u32) -> u32 {
    #[action("Produce a wire.")]
    |&input| -> shared { *input };

    #[action("Produce the same wire name again.")]
    |shared| -> shared { shared };

    #[action("Use the wire.")]
    |shared| -> result { shared };
}

fn main() {}
