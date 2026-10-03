use kaalang::kaalang;

#[kaalang]
fn invalid() -> ! {
    #[cycle("Try to describe a continue.")]
    loop {
        #[allow(unused)]
        continue;
    };
}

fn main() {}
