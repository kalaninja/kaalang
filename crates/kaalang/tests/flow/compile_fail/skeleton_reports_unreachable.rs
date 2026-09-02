#![deny(unreachable_code)]

use kaalang::kaalang;

#[kaalang]
fn skeleton(seed: u32) -> u32 {
    #[action("Leave the first body unwritten.")]
    |&seed| -> started { todo!() };

    #[action("Produce the result.")]
    |started| -> result { started };

    #[end]
    |result| {};
}

fn main() {
    let _ = skeleton(1);
}
