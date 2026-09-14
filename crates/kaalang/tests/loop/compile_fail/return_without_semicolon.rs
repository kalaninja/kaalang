use kaalang::kaalang;

#[kaalang]
fn invalid(value: usize) -> usize {
    |value| return value
}

fn main() {}
