use kaalang::kaa;

#[kaa]
fn invalid() -> ! {
    #[kaa::cycle("Repeat forever.")]
    {
        continue;
    };
}

fn main() {}
