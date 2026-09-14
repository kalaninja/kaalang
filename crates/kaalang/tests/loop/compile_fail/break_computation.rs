use kaalang::kaalang;

#[kaalang]
fn invalid() -> usize {
    #[cycle("Try to compute in a transfer.")]
    let result = || {
        break 1;
    };

    |result| return result;
}

fn main() {}
