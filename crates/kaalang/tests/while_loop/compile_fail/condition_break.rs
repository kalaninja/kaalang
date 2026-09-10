use kaalang::kaalang;

#[kaalang]
fn invalid(flag: bool) {
    #[question("Repeat?")]
    while (|flag| { if flag { break; } false }) {}
    #[action("Finish.")]
    let result = || {};
}

fn main() {}
