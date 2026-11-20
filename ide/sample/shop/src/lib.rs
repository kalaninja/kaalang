//! A small shop. `Order` is the focus most cases start from: the types that
//! refer to it live in this file, in other files and modules, in the binary,
//! a test, an example, and the `billing` package.

use kaalang::kaa;

pub mod aliases;
pub mod catalog;
pub mod customers;
pub mod flows;
#[path = "generated/ids.rs"]
pub mod ids;
// One file, two modules: rustc reads it twice, rust-analyzer only in the first.
mod shared;
#[path = "shared.rs"]
mod twin;

pub use ids::OrderNumber;

#[kaa("A **shop order**: what a customer bought, and how it ships.")]
pub struct Order {
    #[kaa::refer(OrderNumber, "The order's number, unique in the shop.")]
    pub number: OrderNumber,
    #[kaa::refer(customers::Customer, "Who placed the order.")]
    pub customer: customers::CustomerId,
    #[kaa::refer(catalog::Line, "Products and quantities.")]
    pub lines: Vec<catalog::Line>,
    #[kaa::value("Ships the next day: the default `express` feature is on.")]
    #[cfg(feature = "express")]
    pub express: bool,
    #[kaa::value("A note for the gift wrap: the `gift` feature is off.")]
    #[cfg(feature = "gift")]
    pub gift_note: String,
    #[kaa::value("The warehouse shelf: the build script sets `warehouse`.")]
    #[cfg(warehouse)]
    pub shelf: u16,
    #[kaa::refer(Self, "The previous revision, kept for audit.")]
    pub previous: Option<Box<Self>>,
}

#[kaa("Why an order is not on its way yet.")]
pub enum Hold {
    #[kaa::case("The payment has not arrived.")]
    AwaitingPayment(#[kaa::refer(Order, "The unpaid order.")] OrderNumber),
    #[kaa::case("The shelf has fewer units than the order needs.")]
    OutOfStock {
        #[kaa::refer(Order, "The order that waits.")]
        order: OrderNumber,
        #[kaa::refer(catalog::Product, "The product that ran out.")]
        product: catalog::Sku,
    },
}

#[kaa("A place on the packing table: an order, or nothing yet.")]
#[derive(Clone, Copy)]
pub union Slot {
    #[kaa::refer(Order, "The order being packed.")]
    pub order: u32,
    #[kaa::value("Free.")]
    pub free: u32,
}

#[kaa("The orders waiting to be packed, oldest first.")]
#[kaa::refer(Order, "Paid orders no shipment has taken yet.")]
pub type Backlog = Vec<OrderNumber>;

#[kaa("Something that happened to an order, with any payload.")]
pub struct Event<T> {
    #[kaa::refer(Order, "The order it happened to.")]
    pub order: OrderNumber,
    #[kaa::value("What happened.")]
    pub payload: T,
}

#[kaa("Wrapping for a gift. The `gift` feature is off, so it draws no card.")]
#[cfg(feature = "gift")]
pub struct GiftWrap {
    #[kaa::refer(Order, "The order to wrap.")]
    pub order: OrderNumber,
}

pub mod payments {
    //! Inline modules: card headers show their paths.

    use kaalang::kaa;

    use crate::{Order, OrderNumber, catalog::Money};

    #[kaa("Money received for an order.")]
    pub struct Payment {
        #[kaa::refer(Order, "The order paid for.")]
        pub order: OrderNumber,
        #[kaa::value("The amount received.")]
        pub amount: Money,
    }

    pub mod methods {
        use kaalang::kaa;

        #[kaa("How a payment arrived.")]
        pub enum Method {
            #[kaa::case("By card, at checkout.")]
            Card,
            #[kaa::case("By bank transfer, with the order's number as reference.")]
            Transfer {
                #[kaa::refer(crate::Order, "The order named in the reference.")]
                order: crate::OrderNumber,
            },
        }
    }
}
