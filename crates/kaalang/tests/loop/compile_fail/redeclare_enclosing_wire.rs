use kaalang::kaalang;

#[kaalang]
fn invalid(count: usize) -> ! {
    #[cycle("Try to redeclare a cycle input.")]
    |count| {
        #[action("Replace the counter.")]
        let count = || 1;
    };
}

fn main() {}
