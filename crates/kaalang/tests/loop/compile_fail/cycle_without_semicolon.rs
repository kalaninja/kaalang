use kaalang::kaalang;

#[kaalang]
fn invalid() -> ! {
    #[cycle("Omit the trailing semicolon.")]
    || {}
}

fn main() {}
