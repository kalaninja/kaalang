#![deny(unused_mut)]

use kaalang::kaa;

#[kaa]
fn unused_mutable_capture(value: u8) -> u8 {
    #[kaa::action("Leave the authored mutable binding unchanged.")]
    let end = |mut value| { value };

    |end| return end;
}

fn main() {}
