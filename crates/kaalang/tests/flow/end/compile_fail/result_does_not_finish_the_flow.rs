use kaalang::kaa;

#[kaa]
fn invalid() {
    #[kaa::action("Produce an ordinary wire.")]
    let result = || {};
}

fn main() {}
