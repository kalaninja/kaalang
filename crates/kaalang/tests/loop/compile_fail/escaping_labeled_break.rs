use kaalang::kaalang;

#[kaalang]
fn invalid(flag: bool) -> usize {
    'outer: loop {
        #[action("Escape.")]
        || {
            break 'outer;
        };
    }
    #[action("Finish.")]
    let end = || 0;
}

fn main() {}
