use kaalang::kaalang;

#[kaalang]
fn invalid(flag: bool) -> usize {
    |missing| loop {};
    #[action("Finish.")]
    let end = || 0;
}

fn main() {}
