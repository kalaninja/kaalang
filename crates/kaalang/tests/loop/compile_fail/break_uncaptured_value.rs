use kaalang::kaalang;

#[kaalang]
fn invalid(flag: bool) -> bool {
    #[cycle("Try to transfer an uncaptured value.")]
    let result = |flag| {
        break flag;
    };

    |result| return result;
}

fn main() {}
