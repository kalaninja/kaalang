use kaalang::kaalang;

#[kaalang]
fn invalid() {
    #[cycle("Repeat forever.")]
    loop {
        #[action("Visit a Rust iterator.")]
        {
            for _ in {
                if std::hint::black_box(true) {
                    continue;
                }
                0..1
            } {}
        };
    };
}

fn main() {}
