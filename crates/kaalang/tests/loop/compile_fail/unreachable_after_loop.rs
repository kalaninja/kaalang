use kaalang::kaalang;

#[kaalang]
fn invalid() {
    loop {}

    #[action("Finish.")]
    let end = || {};
}

fn main() {}
