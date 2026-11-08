use kaalang::kaa;

#[kaa]
fn invalid(count: usize) -> ! {
    #[kaa::cycle("Try to shadow an outer wire.")]
    loop {
        #[kaa::action("Replace the counter.")]
        let count = || 1;
    };
}

fn main() {}
