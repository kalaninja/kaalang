use kaalang::kaa;

#[kaa]
fn invalid() -> ! {
    #[kaa::cycle("Try a labeled body.")]
    || 'body: loop {};
}

fn main() {}
