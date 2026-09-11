use kaalang::kaalang;

#[kaalang]
fn invalid(flag: bool) -> usize {
    #[allow(unused)]
    |flag| loop {};
    #[action("Finish.")]
    let end = || 0;
}

fn main() {}
