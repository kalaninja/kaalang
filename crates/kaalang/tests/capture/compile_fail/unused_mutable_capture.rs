#![deny(unused_mut)]

use kaalang::kaalang;

#[kaalang]
fn unused_mutable_capture(value: u8) -> u8 {
    #[action("Leave the authored mutable binding unchanged.")]
    let result = |mut value| { value };
}

fn main() {}
