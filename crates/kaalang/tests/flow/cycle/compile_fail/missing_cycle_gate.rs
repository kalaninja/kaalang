use kaalang::kaa;

#[kaa]
fn invalid() -> ! {
    #[kaa::cycle("Gate on a missing wire.")]
    |missing| loop {};
}

fn main() {}
