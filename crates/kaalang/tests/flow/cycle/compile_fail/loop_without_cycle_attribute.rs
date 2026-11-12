use kaalang::kaa;

#[kaa]
fn invalid() -> ! {
    #[kaa::action("Repeat forever.")]
    loop {};
}

fn main() {}
