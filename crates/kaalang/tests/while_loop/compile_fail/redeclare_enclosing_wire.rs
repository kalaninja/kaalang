use kaalang::kaalang;

#[kaalang]
fn invalid(flag: bool) {
    #[question("Repeat?")]
    while (|flag| flag) {
        #[action("Shadow the input.")]
        let flag = || false;
    }
    #[action("Finish.")]
    let end = || {};
}

fn main() {}
