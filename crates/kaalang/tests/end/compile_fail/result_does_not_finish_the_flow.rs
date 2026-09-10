use kaalang::kaalang;

#[kaalang]
fn invalid() {
    #[action("Produce an ordinary wire.")]
    let result = || {};
}

fn main() {}
