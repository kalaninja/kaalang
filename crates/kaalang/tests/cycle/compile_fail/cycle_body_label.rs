use kaalang::kaalang;

#[kaalang]
fn invalid() -> ! {
    #[cycle("Try a labeled body.")]
    || 'body: {};
}

fn main() {}
