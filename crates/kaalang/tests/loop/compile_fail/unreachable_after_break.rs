use kaalang::kaalang;

#[kaalang]
fn invalid(flag: bool) -> usize {
    loop {
        break;
        #[action("Too late.")]
        || {};
    }
    #[action("Finish.")]
    let end = || 0;
}

fn main() {}
