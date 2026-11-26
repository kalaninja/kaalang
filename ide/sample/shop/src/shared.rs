//! Included as `shared` and, through `#[path]`, as `twin`. The CLI draws a
//! card for each; the editor draws the copy rust-analyzer reads.

use kaalang::kaa;

#[kaa("A check of an order, by someone other than who took it.")]
pub struct Audit {
    #[kaa::refer(crate::Order, "The order checked.")]
    pub order: crate::OrderNumber,
}
