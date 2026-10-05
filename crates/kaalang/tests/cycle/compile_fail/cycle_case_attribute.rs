use kaalang::kaalang;

#[kaalang]
fn invalid() -> ! {
    #[cycle("Try to add a case.")]
    #[case("Unsupported.")]
    || loop {};
}

fn main() {}
