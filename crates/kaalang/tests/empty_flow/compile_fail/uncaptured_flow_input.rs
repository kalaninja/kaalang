use kaalang::kaalang;

#[kaalang]
fn invalid(value: u8) {
    #[end]
    || {};
}

fn main() {}
