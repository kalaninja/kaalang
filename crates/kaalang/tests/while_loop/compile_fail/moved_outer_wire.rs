use kaalang::kaalang;

#[kaalang]
fn invalid(text: String) {
    #[question("Repeat?")]
    while (|text| !text.is_empty()) {}
    #[action("Finish.")]
    let end = || {};
}

fn main() {}
