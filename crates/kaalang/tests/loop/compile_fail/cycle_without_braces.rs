use kaalang::kaalang;

#[kaalang]
fn invalid() {
    #[cycle("Omit the body braces.")]
    || continue;

    return;
}

fn main() {}
