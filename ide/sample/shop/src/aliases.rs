//! A renamed import: rust-analyzer's reference search stops at `as`, so the
//! editor finds `Refund` through the target's definition.

use kaalang::kaa;

pub use crate::Order as Purchase;

#[kaa("Money back for an order.")]
pub struct Refund {
    #[kaa::refer(Purchase, "The order refunded.")]
    pub purchase: crate::OrderNumber,
}
