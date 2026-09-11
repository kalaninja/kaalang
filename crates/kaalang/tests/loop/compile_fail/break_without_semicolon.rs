use kaalang::kaalang;

#[kaalang]
fn invalid(flag: bool) {
    loop {
        |flag| break
    }
    #[action("Finish.")]
    let end = || {};
}

fn main() {}
