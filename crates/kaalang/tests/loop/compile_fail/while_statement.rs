use kaalang::kaalang;

#[kaalang]
fn invalid(flag: bool) -> usize {
    #[question("Repeat?")]
    while (|flag| flag) {}
    #[action("Finish.")]
    let end = || 0;
}

fn main() {}
