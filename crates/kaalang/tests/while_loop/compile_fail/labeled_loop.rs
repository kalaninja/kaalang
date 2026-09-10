use kaalang::kaalang;

#[kaalang]
fn invalid(flag: bool) {
    #[question("Repeat?")]
    'search: while (|flag| flag) {}
    #[action("Finish.")]
    let result = || {};
}

fn main() {}
