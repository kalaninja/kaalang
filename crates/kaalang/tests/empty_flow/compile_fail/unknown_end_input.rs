use kaalang::kaalang;

#[kaalang]
fn invalid() -> u8 {
    #[end]
    |unknown| {};
}

fn main() {}
