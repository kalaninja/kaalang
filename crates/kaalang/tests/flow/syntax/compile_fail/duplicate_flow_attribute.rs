use kaalang::kaa;

#[kaa]
#[cfg_attr(all(), kaa)]
fn doubled(value: u8) -> u8 {
    |value| return value;
}

fn main() {}
