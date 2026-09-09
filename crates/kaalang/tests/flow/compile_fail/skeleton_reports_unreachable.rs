#![deny(unreachable_code)]

use kaalang::kaalang;

#[kaalang]
fn skeleton(seed: u32) -> u32 {
    #[action("Leave the first body unwritten.")]
    let started = |&seed| { todo!() };

    #[action("Produce the result.")]
    let result = |started| { started };
}

fn main() {
    let _ = skeleton(1);
}
