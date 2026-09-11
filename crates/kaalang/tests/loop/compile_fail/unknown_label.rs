use kaalang::kaalang;

#[kaalang]
fn invalid(flag: bool) -> usize {
    loop {
        break 'missing;
    }
    #[action("Finish.")]
    let end = || 0;
}

fn main() {}
