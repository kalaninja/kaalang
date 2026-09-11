use kaalang::kaalang;

#[kaalang]
fn invalid(flag: bool) -> usize {
    loop {
        break 1;
    }
    #[action("Finish.")]
    let end = || 0;
}

fn main() {}
