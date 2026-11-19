use kaalang::kaa;

#[kaa]
fn read(
    #[cfg(all(), any())] _first: u8,
    #[cfg_attr(all(), , allow(unused))] _second: u8,
    #[cfg(all())] value: u32,
) -> u32 {
    |value| return value;
}

fn main() {}
