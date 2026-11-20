//! A directory module, with its submodule in `product.rs`.

use kaalang::kaa;

pub mod product;

pub use product::{Product, Sku};

/// An amount in the shop's currency, in cents.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Money(pub u64);

#[kaa("One product in an order, with its quantity.")]
pub struct Line {
    #[kaa::refer(Product, "The product ordered.")]
    pub product: Sku,
    #[kaa::value("How many units.")]
    pub quantity: u32,
    #[kaa::value("The price of one unit when the order was placed.")]
    pub price: Money,
}
