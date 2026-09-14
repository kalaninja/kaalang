use kaalang::kaalang;

#[kaalang]
fn invalid(flag: bool) -> ! {
    |flag| loop {};
}

fn main() {}
