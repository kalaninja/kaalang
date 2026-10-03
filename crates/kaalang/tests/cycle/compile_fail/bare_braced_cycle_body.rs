use kaalang::kaalang;

#[kaalang]
fn invalid() -> ! {
    #[cycle("Repeat forever.")]
    {
        continue;
    };
}

fn main() {}
