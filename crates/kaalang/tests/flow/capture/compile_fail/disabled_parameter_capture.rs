use kaalang::kaa;

#[kaa]
fn read(#[cfg(any())] value: u8) -> u8 {
    |value| return value;
}

fn main() {}
