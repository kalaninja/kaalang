use kaalang::kaalang;

#[kaalang]
fn invalid() {
    #[cycle("Repeat the outer cycle.")]
    {
        #[cycle("Spin forever.")]
        {
            continue;
        };

        #[action("Too late.")]
        || {};

        continue;
    };
}

fn main() {}
