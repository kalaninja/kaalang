use kaalang::kaalang;

#[kaalang]
fn invalid(flag: bool) -> usize {
    #[question("Run?")]
    let (_run, _skip) = |flag| flag;
    loop {}
    #[action("Finish.")]
    let end = || 0;
}

fn main() {}
