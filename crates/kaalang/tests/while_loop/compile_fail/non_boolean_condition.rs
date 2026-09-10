use kaalang::kaalang;

#[kaalang]
fn invalid(flag: bool) {
    #[question("Repeat?")]
    while (|flag| usize::from(flag)) {}
    #[action("Finish.")]
    let result = || {};
}

fn main() {}
