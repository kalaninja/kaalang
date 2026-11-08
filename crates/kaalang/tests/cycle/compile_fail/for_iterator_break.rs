use kaalang::kaa;

#[kaa]
fn invalid() {
    #[kaa::cycle("Repeat forever.")]
    loop {
        #[kaa::action("Visit a Rust iterator.")]
        {
            for _ in {
                if std::hint::black_box(true) {
                    break;
                }
                0..1
            } {}
        };
    };
}

fn main() {}
