use kaalang::kaalang;

#[kaalang]
fn invalid() -> ! {
    #[cycle("Do nothing at all.")]
    loop {};
}

fn main() {}
