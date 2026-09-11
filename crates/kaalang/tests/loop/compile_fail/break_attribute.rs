use kaalang::kaalang;

#[kaalang]
fn invalid(flag: bool) -> usize {
    loop {
        #[action("Exit.")]
        break;
    }
    #[action("Finish.")]
    let end = || 0;
}

fn main() {}
