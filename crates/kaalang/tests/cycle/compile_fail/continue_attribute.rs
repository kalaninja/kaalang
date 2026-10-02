use kaalang::kaalang;

#[kaalang]
fn invalid() -> ! {
    #[cycle("Try to describe a continue.")]
    {
        #[allow(unused)]
        continue;
    };
}

fn main() {}
