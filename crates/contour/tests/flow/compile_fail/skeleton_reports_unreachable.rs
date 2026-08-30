#![deny(unreachable_code)]

use contour::contour;

#[contour]
fn skeleton(seed: u32) -> u32 {
    #[action("Leave the first body unwritten.")]
    |&seed| -> started { todo!() };

    #[action("Finish the flow.")]
    |started| -> result { started };
}

fn main() {
    let _ = skeleton(1);
}
