use kaalang::kaa;

#[kaa]
fn invalid() {
    #[kaa::cycle("Repeat forever.")]
    loop {
        continue;
    };

    return;
}

fn main() {}
