use kaalang::kaalang;

#[kaalang]
fn invalid(input: u32) -> u32 {
    #[action("Produce a wire.")]
    let shared = |&input| { *input };

    #[action("Produce the same wire name again.")]
    let shared = |shared| { shared };

    |shared| return shared;
}

fn main() {}
