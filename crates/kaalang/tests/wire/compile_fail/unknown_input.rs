use kaalang::kaalang;

#[kaalang]
fn invalid(input: u32) -> u32 {
    |future| return future;

    #[action("Declare the wire too late.")]
    let future = |input| { input };

}

fn main() {}
