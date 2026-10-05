use kaalang::kaalang;

#[kaalang]
fn invalid(count: usize) -> ! {
    #[cycle("Try to shadow an outer wire.")]
    loop {
        #[action("Replace the counter.")]
        let count = || 1;
    };
}

fn main() {}
