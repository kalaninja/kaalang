use kaalang::kaalang;

#[kaalang]
fn invalid() {
    #[cycle("Repeat the outer cycle.")]
    loop {
        #[cycle("Spin forever.")]
        loop {
            continue;
        };

        #[action("Too late.")]
        || {};

        continue;
    };
}

fn main() {}
