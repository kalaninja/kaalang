use kaalang::kaalang;

#[kaalang]
fn invalid(flag: bool) -> usize {
    'first: loop {
        break 'first;
    }
    loop {
        break 'first;
    }
    #[action("Finish.")]
    let end = || 0;
}

fn main() {}
