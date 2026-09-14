use kaalang::kaalang;

#[kaalang]
fn invalid() {
    #[cycle("Try an attributed break.")]
    || {
        #[action("Exit.")]
        break;
    };

    return;
}

fn main() {}
