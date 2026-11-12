use kaalang::kaa;

#[kaa]
fn invalid(first: bool, second: bool) -> ! {
    #[kaa::cycle("Try two gates.")]
    |first, second| loop {};
}

fn main() {}
