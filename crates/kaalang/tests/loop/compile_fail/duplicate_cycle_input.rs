use kaalang::kaalang;

#[kaalang]
fn invalid(flag: bool) -> ! {
    #[cycle("Try duplicate cycle captures.")]
    |flag, flag| {};
}

fn main() {}
