use kaalang::kaalang;

#[kaalang]
fn invalid(input: u32) -> u32 {
    #[action("Use a future wire.")]
    |future| -> result { future };

    #[action("Declare the wire too late.")]
    |input| -> future { input };
}

fn main() {}
