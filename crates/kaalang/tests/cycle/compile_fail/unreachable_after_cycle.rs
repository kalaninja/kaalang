use kaalang::kaalang;

#[kaalang]
fn invalid() {
    #[cycle("Repeat forever.")]
    loop {
        continue;
    };

    return;
}

fn main() {}
