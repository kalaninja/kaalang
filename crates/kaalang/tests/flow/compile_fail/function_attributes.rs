#![deny(unused_must_use)]

use kaalang::kaalang;

/// Doc comment above the attribute.
#[must_use]
#[kaalang]
#[inline]
pub fn doubled(value: i32) -> i32 {
    #[action("Double the value.")]
    |value| -> doubled { value * 2 };

    #[end]
    |doubled| {};
}

fn main() {
    // Fails only while `#[kaalang]` still re-emits the function's own attributes.
    doubled(1);
}
