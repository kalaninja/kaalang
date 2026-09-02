use kaalang::kaalang;

#[kaalang]
fn missing_kind(value: i32) -> i32 {
    |value| -> doubled { value * 2 };
}

fn main() {}
