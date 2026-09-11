use kaalang::kaalang;

#[kaalang]
fn invalid(flag: bool) -> usize {
    break;
    #[action("Finish.")]
    let end = || 0;
}

fn main() {}
