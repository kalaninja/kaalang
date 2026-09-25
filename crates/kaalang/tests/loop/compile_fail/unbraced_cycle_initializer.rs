use kaalang::kaalang;

#[kaalang]
fn invalid() -> u32 {
    #[cycle("Try a cycle body without braces.")]
    let result = 1;

    |result| return result;
}

fn main() {}
