use kaalang::kaalang;

#[kaalang]
fn invalid(flag: bool) -> usize {
    loop {
        |missing| break;
    }
    #[action("Finish.")]
    let end = || 0;
}

fn main() {}
