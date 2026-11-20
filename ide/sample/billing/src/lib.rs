//! Another package: its cards stand in a row of their own, under their crate's
//! name.

use kaalang::kaa;
use shop::{Order, OrderNumber, catalog::Money};

#[kaa("A bill for an order.")]
pub struct Invoice {
    #[kaa::refer(Order, "The order billed.")]
    pub order: OrderNumber,
    #[kaa::value("The amount due.")]
    pub total: Money,
}

/// A flow that calls a flow of another package.
#[kaa]
pub fn bill(order: &Order) -> Invoice {
    #[kaa::call("Add up the order.")]
    let total = |order| shop::flows::total(order);

    #[kaa::action("🧾 Write the bill.")]
    let invoice = |order, total| Invoice {
        order: order.number,
        total,
    };

    |invoice| return invoice;
}
