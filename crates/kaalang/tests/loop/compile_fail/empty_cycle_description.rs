use kaalang::kaalang;

#[kaalang]
fn invalid() -> ! {
    #[cycle("")]
    || {};
}

fn main() {}
