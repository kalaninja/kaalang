use kaalang::kaalang;

#[kaalang]
fn invalid() -> ! {
    #[cycle("Try to continue explicitly.")]
    || {
        continue;
    };
}

fn main() {}
