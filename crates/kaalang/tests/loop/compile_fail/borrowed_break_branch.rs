use kaalang::kaalang;

#[kaalang]
fn invalid(flag: bool) {
    #[cycle("Check the flag.")]
    |flag| {
        #[question("Exit?")]
        let (done, _again) = |flag| flag;

        |&done| break;
    };

    return;
}

fn main() {}
