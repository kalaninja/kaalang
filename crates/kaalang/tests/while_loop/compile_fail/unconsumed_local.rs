use kaalang::kaalang;

#[kaalang]
fn invalid(flag: bool) {
    #[question("Repeat?")]
    while (|flag| flag) {
        #[action("Create a local wire.")]
        let local = || 1;
    }
    #[action("Finish.")]
    let end = || {};
}

fn main() {}
