use kaalang::kaalang;

#[kaalang]
fn invalid(flag: bool) -> usize {
    |flag, flag| loop {};
    #[action("Finish.")]
    let end = || 0;
}

fn main() {}
