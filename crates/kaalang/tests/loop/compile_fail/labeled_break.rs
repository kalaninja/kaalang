use kaalang::kaalang;

#[kaalang]
fn invalid() {
    #[cycle("Try a labeled break.")]
    || {
        break 'outer;
    };

    return;
}

fn main() {}
