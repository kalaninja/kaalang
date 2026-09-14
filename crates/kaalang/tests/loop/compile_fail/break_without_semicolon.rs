use kaalang::kaalang;

#[kaalang]
fn invalid(flag: bool) -> bool {
    #[cycle("Try a break without a semicolon.")]
    let result = |flag| {
        #[action("Keep the cycle body braced.")]
        |&flag| {};

        |flag| break flag
    };

    |result| return result;
}

fn main() {}
