use kaalang::kaalang;

#[kaalang]
fn invalid(input: u32) -> u32 {
    #[action("Use a future wire.")]
    let end = |future| { future };

    #[action("Declare the wire too late.")]
    let future = |input| { input };
}

fn main() {}
