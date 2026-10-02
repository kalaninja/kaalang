use kaalang::kaalang;

#[kaalang]
fn invalid(flag: bool) -> ! {
    #[cycle("Repeat through two transfers.")]
    {
        #[question("Which route?")]
        let (first, second) = |flag| flag;

        |first| continue;
        |second| continue;
    };
}

fn main() {}
