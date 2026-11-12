use kaalang::kaa;

#[kaa]
fn invalid(input: u32) -> u32 {
    #[kaa::end]
    |input| {};

    |input| return input;
}

fn main() {}
