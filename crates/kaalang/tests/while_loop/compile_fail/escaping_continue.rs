use kaalang::kaalang;

#[kaalang]
fn invalid(flag: bool) {
    #[question("Repeat?")]
    while (|flag| flag) {
        #[action("Escape.")]
        || { continue; };
    }
    #[action("Finish.")]
    let result = || {};
}

fn main() {}
