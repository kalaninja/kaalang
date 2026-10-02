use kaalang::kaalang;

#[kaalang]
fn invalid(count: usize) -> ! {
    #[cycle("Try to shadow an outer wire.")]
    {
        #[action("Replace the counter.")]
        let count = || 1;
    };
}

fn main() {}
