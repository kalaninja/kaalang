use kaalang::kaalang;

#[kaalang]
fn invalid() -> ! {
    #[cycle("Try an unsupported attribute.")]
    #[allow(unused)]
    || {};
}

fn main() {}
