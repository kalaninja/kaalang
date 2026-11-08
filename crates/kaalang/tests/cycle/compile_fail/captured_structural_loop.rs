use kaalang::kaa;

#[kaa]
fn invalid(flag: bool) -> ! {
    |flag| loop {};
}

fn main() {}
