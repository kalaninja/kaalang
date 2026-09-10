use kaalang::kaalang;

#[kaalang]
fn invalid() {
    loop {
        break;
    }

    #[action("Finish.")]
    let end = || {};
}

fn main() {}
