use kaalang::kaa;

#[kaa]
fn invalid() {
    #[kaa::cycle("Repeat the outer cycle.")]
    loop {
        #[kaa::cycle("Spin forever.")]
        loop {
            continue;
        };

        #[kaa::action("Too late.")]
        || {};

        continue;
    };
}

fn main() {}
