use kaalang::kaalang;

#[kaalang]
fn invalid(flag: bool) -> ! {
    #[cycle("Try to borrow the gate.")]
    |&flag| loop {};
}

fn main() {}
