use kaalang::kaa;

/// A customer's key. A plain type, without `#[kaa]`: it can still be a focus
/// and a relationship's target.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CustomerId(pub u32);

#[kaa("A person who buys from the shop.")]
pub struct Customer {
    #[kaa::refer(CustomerId, "The customer's key.")]
    pub id: CustomerId,
    #[kaa::value("The name on the parcel.")]
    pub name: String,
    #[kaa::refer(crate::Order, "The latest order, once there is one.")]
    pub latest: Option<crate::OrderNumber>,
}
