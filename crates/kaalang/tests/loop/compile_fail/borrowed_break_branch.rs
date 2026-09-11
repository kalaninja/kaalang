use kaalang::kaalang;

#[kaalang]
fn invalid(flag: bool) -> usize {
    loop {
        #[question("Exit?")]
        let (done, _again) = |flag| flag;
        |&done| break;
    }
    #[action("Finish.")]
    let end = || 0;
}

fn main() {}
