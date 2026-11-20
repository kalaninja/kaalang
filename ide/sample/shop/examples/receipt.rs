//! An example: its types draw cards in a row of their own.

use kaalang::kaa;
use shop::Order;

#[kaa("A printed receipt, borrowing the order it lists.")]
struct Receipt<'a> {
    #[kaa::refer(Order, "The order listed.")]
    order: &'a Order,
}

fn main() {}
