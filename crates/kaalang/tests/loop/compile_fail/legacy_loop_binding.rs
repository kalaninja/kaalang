use kaalang::kaalang;

#[kaalang]
fn invalid() -> ! {
    #[r#loop("Use the legacy attribute form.")]
    let result = || {};
}

fn main() {}
