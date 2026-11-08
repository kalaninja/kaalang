use kaalang::kaa;

#[kaa]
fn invalid() -> ! {
    #[kaa::cycle("Try an unsupported attribute.")]
    #[allow(unused)]
    || loop {};
}

fn main() {}
