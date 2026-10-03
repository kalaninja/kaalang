use kaalang::kaalang;

#[kaalang]
fn invalid() -> u32 {
    #[cycle("Try a cycle body without a loop.")]
    let result = 1;

    |result| return result;
}

fn main() {}
