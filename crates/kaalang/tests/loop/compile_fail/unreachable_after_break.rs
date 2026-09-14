use kaalang::kaalang;

#[kaalang]
fn invalid() {
    #[cycle("Break before later work.")]
    || {
        break;

        #[action("Too late.")]
        || {};
    };

    return;
}

fn main() {}
