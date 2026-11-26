use kaalang::kaa;

use super::Money;

/// A stock-keeping unit: the key of a product.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Sku(pub u32);

#[kaa("A product the shop sells.")]
pub struct Product {
    #[kaa::refer(Sku, "The product's key.")]
    pub sku: Sku,
    #[kaa::value("The price of one unit today.")]
    pub price: Money,
    #[kaa::value("Units on the shelf.")]
    pub stocked: u32,
}
