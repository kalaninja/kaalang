use kaalang::kaa;

#[kaa]
fn invalid() -> ! {
    #[kaa::cycle("")]
    || loop {};
}

fn main() {}
