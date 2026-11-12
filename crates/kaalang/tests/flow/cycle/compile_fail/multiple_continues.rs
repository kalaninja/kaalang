use kaalang::kaa;

#[kaa]
fn invalid(flag: bool) -> ! {
    #[kaa::cycle("Repeat through two transfers.")]
    loop {
        #[kaa::question("Which route?")]
        let (first, second) = |flag| flag;

        |first| continue;
        |second| continue;
    };
}

fn main() {}
