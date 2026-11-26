//! A module from a `#[path]` outside the default layout, with a flow.

use kaalang::kaa;

/// An order's number. A plain type, without `#[kaa]`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OrderNumber(pub u32);

#[kaa]
pub fn next_number(last: OrderNumber) -> OrderNumber {
    #[kaa::action("🔢 Take the number after the last one.")]
    let next = |last| OrderNumber(last.0 + 1);

    |next| return next;
}
