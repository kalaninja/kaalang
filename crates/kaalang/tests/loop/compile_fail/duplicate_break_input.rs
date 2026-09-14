use kaalang::kaalang;

#[kaalang]
fn invalid(flag: bool) {
    #[cycle("Try duplicate break captures.")]
    |flag| {
        |flag, flag| break;
    };

    return;
}

fn main() {}
