use kaalang::kaa;

#[kaa]
fn invalid() -> ! {
    #[kaa::cycle("Try to describe a continue.")]
    loop {
        #[allow(unused)]
        continue;
    };
}

fn main() {}
