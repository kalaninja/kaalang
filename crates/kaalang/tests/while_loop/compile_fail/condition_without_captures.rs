use kaalang::kaalang;

#[kaalang]
fn invalid(flag: bool) {
    #[question("Repeat?")]
    while flag {}
    #[action("Finish.")]
    let end = || {};
}

fn main() {}
