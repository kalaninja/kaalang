use kaalang::kaalang;

#[kaalang]
fn invalid(flag: bool) {
    while (|flag| flag) {}
    #[action("Finish.")]
    let result = || {};
}

fn main() {}
