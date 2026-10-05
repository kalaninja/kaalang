use kaalang::kaalang;

#[kaalang]
fn invalid(flag: bool) -> bool {
    #[cycle("Try to leave through a break.")]
    let result = |flag| loop {
        |flag| break flag;
    };

    |result| return result;
}

fn main() {}
