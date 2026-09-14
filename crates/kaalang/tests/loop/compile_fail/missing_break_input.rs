use kaalang::kaalang;

#[kaalang]
fn invalid() {
    #[cycle("Capture a missing break input.")]
    || {
        |missing| break;
    };

    return;
}

fn main() {}
