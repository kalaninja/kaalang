use kaalang::kaalang;

#[kaalang]
fn invalid() -> u8 {
    #[end]
    || {};
}

fn main() {}
