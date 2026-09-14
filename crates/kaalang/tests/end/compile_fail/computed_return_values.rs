use kaalang::kaalang;

fn identity(value: u32) -> u32 {
    value
}

#[kaalang]
fn computed(value: u32) -> u32 {
    |value| return value + 1;
}

#[kaalang]
fn called(value: u32) -> u32 {
    |value| return identity(value);
}

fn main() {}
