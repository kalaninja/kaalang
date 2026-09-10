use kaalang::kaalang;

#[kaalang]
fn invalid(flag: bool) {
    #[question("Repeat?")]
    while let Some(_) = Some(flag) {}
    #[action("Finish.")]
    let end = || {};
}

fn main() {}
