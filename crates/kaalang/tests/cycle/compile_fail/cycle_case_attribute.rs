use kaalang::kaa;

#[kaa]
fn invalid() -> ! {
    #[kaa::cycle("Try to add a case.")]
    #[kaa::case("Unsupported.")]
    || loop {};
}

fn main() {}
