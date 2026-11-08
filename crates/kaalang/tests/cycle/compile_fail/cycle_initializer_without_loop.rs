use kaalang::kaa;

#[kaa]
fn invalid() -> u32 {
    #[kaa::cycle("Try a cycle body without a loop.")]
    let result = 1;

    |result| return result;
}

fn main() {}
