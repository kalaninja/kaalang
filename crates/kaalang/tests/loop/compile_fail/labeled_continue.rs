use kaalang::kaalang;

#[kaalang]
fn invalid() -> ! {
    #[cycle("Try to continue a label.")]
    {
        continue 'outer;
    };
}

fn main() {}
