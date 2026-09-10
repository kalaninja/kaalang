use kaalang::kaalang;

#[kaalang]
fn invalid() -> ! {
    #[action("Repeat forever.")]
    loop {}
}

fn main() {}
