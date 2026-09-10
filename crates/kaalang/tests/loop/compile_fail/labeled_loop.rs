use kaalang::kaalang;

#[kaalang]
fn invalid() -> ! {
    'repeat: loop {}
}

fn main() {}
