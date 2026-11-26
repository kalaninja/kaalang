//! An integration test: its types draw cards in a row of their own.

use kaalang::kaa;
use shop::{Order, OrderNumber};

#[kaa("A basket a checkout test fills.")]
struct Basket {
    #[kaa::refer(Order, "The order the basket became.")]
    order: OrderNumber,
}

#[test]
fn a_number_follows_the_last() {
    assert_eq!(shop::ids::next_number(OrderNumber(1)), OrderNumber(2));
}
