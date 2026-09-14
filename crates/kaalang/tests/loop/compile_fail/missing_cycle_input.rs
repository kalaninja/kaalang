use kaalang::kaalang;

#[kaalang]
fn invalid() -> ! {
    #[cycle("Capture a missing cycle input.")]
    |missing| {};
}

fn main() {}
