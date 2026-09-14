use kaalang::kaalang;

#[kaalang]
fn invalid(flag: bool) -> ! {
    #[question("Run?")]
    let (_run, _skip) = |flag| flag;

    #[cycle("Enter without selecting a branch.")]
    || {};
}

fn main() {}
