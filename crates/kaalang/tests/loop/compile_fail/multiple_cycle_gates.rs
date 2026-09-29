use kaalang::kaalang;

#[kaalang]
fn invalid(first: bool, second: bool) -> ! {
    #[cycle("Try two gates.")]
    |first, second| {};
}

fn main() {}
