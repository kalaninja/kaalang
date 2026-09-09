use kaalang::kaalang;

#[kaalang]
fn missing_kind(value: i32) -> i32 {
    let doubled = |value| { value * 2 };
}

fn main() {}
