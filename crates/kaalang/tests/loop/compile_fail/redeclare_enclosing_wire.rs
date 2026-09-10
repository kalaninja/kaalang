use kaalang::kaalang;

#[kaalang]
fn invalid(count: usize) -> ! {
    loop {
        #[action("Replace the counter.")]
        let count = || 1;
    }
}

fn main() {}
