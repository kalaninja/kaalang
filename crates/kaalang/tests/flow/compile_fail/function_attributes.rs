#![deny(unused_must_use)]

use kaalang::kaa;

/// Doc comment above the attribute.
#[must_use]
#[kaa]
#[inline]
pub fn doubled(value: i32) -> i32 {
    #[kaa::action("Double the value.")]
    let end = |value| { value * 2 };

    |end| return end;
}

fn main() {
    // Fails only while `#[kaa]` still re-emits the function's own attributes.
    doubled(1);
}
