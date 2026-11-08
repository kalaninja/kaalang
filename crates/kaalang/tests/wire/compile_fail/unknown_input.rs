use kaalang::kaa;

#[kaa]
fn invalid(input: u32) -> u32 {
    |future| return future;

    #[kaa::action("Declare the wire too late.")]
    let future = |input| { input };

}

fn main() {}
