use kaalang::kaalang;

#[kaalang]
fn invalid() {
    #[cycle("Omit the body braces.")]
    || break;

    return;
}

fn main() {}
