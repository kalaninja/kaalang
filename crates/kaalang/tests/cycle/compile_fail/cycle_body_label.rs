use kaalang::kaalang;

#[kaalang]
fn invalid() -> ! {
    #[cycle("Try a labeled body.")]
    || 'body: loop {};
}

fn main() {}
