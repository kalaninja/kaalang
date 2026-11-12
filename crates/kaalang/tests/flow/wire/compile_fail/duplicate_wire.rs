use kaalang::kaa;

#[kaa]
fn invalid(input: u32) -> u32 {
    #[kaa::action("Produce a wire.")]
    let shared = |&input| { *input };

    #[kaa::action("Produce the same wire name again.")]
    let shared = |shared| { shared };

    |shared| return shared;
}

fn main() {}
