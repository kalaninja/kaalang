use kaalang::kaa;

#[kaa]
fn invalid() -> ! {
    #[kaa::cycle("Try to continue a label.")]
    loop {
        continue 'outer;
    };
}

fn main() {}
