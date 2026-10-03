use kaalang::kaalang;

#[kaalang]
fn invalid() -> ! {
    #[cycle("Try to continue a label.")]
    loop {
        continue 'outer;
    };
}

fn main() {}
