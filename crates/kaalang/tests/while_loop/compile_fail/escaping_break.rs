use kaalang::kaalang;

#[kaalang]
fn invalid(flag: bool) {
    #[question("Repeat?")]
    while (|flag| flag) {
        #[action("Escape.")]
        || { break; };
    }
    #[action("Finish.")]
    let result = || {};
}

fn main() {}
