use kaalang::kaalang;

#[kaalang]
fn invalid() -> ! {
    #[cycle("Do nothing at all.")]
    {};
}

fn main() {}
