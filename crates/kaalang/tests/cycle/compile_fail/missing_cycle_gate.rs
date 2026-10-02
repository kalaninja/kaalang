use kaalang::kaalang;

#[kaalang]
fn invalid() -> ! {
    #[cycle("Gate on a missing wire.")]
    |missing| {};
}

fn main() {}
