use kaalang::kaalang;

#[kaalang]
fn invalid(flag: bool) {
    loop {
        #[question("Repeat?")]
        let (again, done) = |flag| flag;
        |done| break;
        #[action("Create an unused local wire.")]
        let local = |again| 1;
    }
    #[action("Finish.")]
    let end = || {};
}

fn main() {}
