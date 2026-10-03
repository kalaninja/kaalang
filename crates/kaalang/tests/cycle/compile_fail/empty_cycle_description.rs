use kaalang::kaalang;

#[kaalang]
fn invalid() -> ! {
    #[cycle("")]
    || loop {};
}

fn main() {}
