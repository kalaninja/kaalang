use kaalang::kaa;

#[kaa]
fn invalid(flag: bool) -> ! {
    #[kaa::cycle("Try to borrow the gate.")]
    |&flag| loop {};
}

fn main() {}
