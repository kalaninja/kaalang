#![deny(unused_must_use)]

use kaalang::kaalang;

/// Doc comment above the attribute.
#[must_use]
#[kaalang]
#[inline]
pub fn doubled(value: i32) -> i32 {
    #[action("Double the value.")]
    let result = |value| { value * 2 };
}

fn main() {
    // Fails only while `#[kaalang]` still re-emits the function's own attributes.
    doubled(1);
}
