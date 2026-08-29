#![deny(unused_must_use)]

use contour::contour;

/// Doc comment above the attribute.
#[must_use]
#[contour]
#[inline]
pub fn doubled(value: i32) -> i32 {
    #[action("Double the value.")]
    |value| -> doubled { value * 2 };
}

fn main() {
    // Fails only while `#[contour]` still re-emits the function's own attributes.
    doubled(1);
}
