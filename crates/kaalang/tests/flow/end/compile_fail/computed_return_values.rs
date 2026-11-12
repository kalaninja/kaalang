use kaalang::kaa;

fn identity(value: u32) -> u32 {
    value
}

#[kaa]
fn computed(value: u32) -> u32 {
    |value| return value + 1;
}

#[kaa]
fn called(value: u32) -> u32 {
    |value| return identity(value);
}

fn main() {}
