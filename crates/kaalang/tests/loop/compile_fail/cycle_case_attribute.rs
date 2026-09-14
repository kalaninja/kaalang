use kaalang::kaalang;

#[kaalang]
fn invalid() -> ! {
    #[cycle("Try to add a case.")]
    #[case("Unsupported.")]
    || {};
}

fn main() {}
