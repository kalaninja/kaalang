use kaalang::kaa;

#[kaa]
fn invalid() -> ! {
    #[kaa::cycle("Do nothing at all.")]
    loop {};
}

fn main() {}
