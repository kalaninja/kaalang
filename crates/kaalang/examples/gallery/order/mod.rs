#![allow(dead_code)]

use kaalang::kaa;

#[kaa("An order a customer placed in the shop.")]
struct Order {
    #[kaa::value("The order's number, unique in the shop.")]
    number: OrderNumber,
    #[kaa::refer(Customer, "Who placed the order.")]
    customer: CustomerId,
    #[kaa::refer(Line, "What was ordered, in the order it was added.")]
    lines: Vec<Line>,
}

#[kaa("The number of an order, unique in the shop.")]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct OrderNumber(u32);

#[kaa("The key of a customer.")]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct CustomerId(u32);

#[kaa("A stock-keeping unit: the key of a product.")]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct Sku(u32);

#[kaa("An amount in the shop's currency, in cents.")]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct Money(u64);

#[kaa("A person who buys from the shop.")]
struct Customer {
    #[kaa::value("The customer's key.")]
    id: CustomerId,
    #[kaa::value("The name on the parcel.")]
    name: String,
    #[kaa::refer(Order, "The customer's latest order, once they placed one.")]
    latest: Option<OrderNumber>,
}

#[kaa("A product the shop sells.")]
struct Product {
    #[kaa::value("The product's key.")]
    sku: Sku,
    #[kaa::value("The name shown in the catalog.")]
    name: String,
    #[kaa::value("The price of one unit today.")]
    price: Money,
    #[kaa::value("Units on the shelf.")]
    stocked: u32,
}

#[kaa("One product in an order, with its quantity.")]
struct Line {
    #[kaa::refer(Product, "The product ordered.")]
    product: Sku,
    #[kaa::value("How many units.")]
    quantity: u32,
    #[kaa::value("The price of one unit when the order was placed.")]
    price: Money,
}

#[kaa("Money received for an order.")]
struct Payment {
    #[kaa::refer(Order, "The order the money is for.")]
    order: OrderNumber,
    #[kaa::value("The amount received.")]
    amount: Money,
    #[kaa::value("The bank's reference for the transfer.")]
    reference: String,
}

#[kaa("A parcel on its way to the customer.")]
struct Shipment {
    #[kaa::refer(Order, "The order in the parcel.")]
    order: OrderNumber,
    #[kaa::value("The carrier's tracking number.")]
    tracking: String,
}

#[kaa("Why an order is not on its way yet.")]
enum Hold {
    #[kaa::case("The payment has not arrived.")]
    AwaitingPayment {
        #[kaa::refer(Order, "The unpaid order.")]
        order: OrderNumber,
    },
    #[kaa::case("The shop has fewer units than the order needs.")]
    OutOfStock {
        #[kaa::refer(Order, "The order that waits.")]
        order: OrderNumber,
        #[kaa::refer(Product, "The product that ran out.")]
        product: Sku,
    },
    #[kaa::case("The customer asked to cancel.")]
    Cancelled {
        #[kaa::refer(Order, "The cancelled order.")]
        order: OrderNumber,
        #[kaa::value("The customer's reason, in their words.")]
        reason: String,
    },
}

#[kaa("A receipt, printed from the order it lists.")]
struct Receipt<'a> {
    #[kaa::refer(Order, "The order the receipt lists, borrowed while it prints.")]
    order: &'a Order,
    #[kaa::value("The amount the customer paid.")]
    paid: Money,
}

#[kaa("The orders waiting to be packed, oldest first.")]
#[kaa::refer(Order, "Paid orders that no shipment has taken yet.")]
type Backlog = Vec<OrderNumber>;

impl Order {
    /// The sum of the lines.
    fn total(&self) -> Money {
        Money(
            self.lines
                .iter()
                .map(|line| line.price.0 * u64::from(line.quantity))
                .sum(),
        )
    }

    /// Why the order waits, given what was paid for it and what the shop has.
    fn hold(&self, payments: &[Payment], catalog: &[Product]) -> Option<Hold> {
        let paid: u64 = payments
            .iter()
            .filter(|payment| payment.order == self.number)
            .map(|payment| payment.amount.0)
            .sum();
        if paid < self.total().0 {
            return Some(Hold::AwaitingPayment { order: self.number });
        }
        let short = self.lines.iter().find(|line| {
            !catalog
                .iter()
                .any(|product| product.sku == line.product && product.stocked >= line.quantity)
        })?;
        Some(Hold::OutOfStock {
            order: self.number,
            product: short.product,
        })
    }
}

pub(super) fn check() {
    let mut catalog = [
        Product {
            sku: Sku(1),
            name: String::from("Teapot"),
            price: Money(2500),
            stocked: 3,
        },
        Product {
            sku: Sku(2),
            name: String::from("Tea"),
            price: Money(900),
            stocked: 0,
        },
    ];
    let order = Order {
        number: OrderNumber(7),
        customer: CustomerId(1),
        lines: vec![
            Line {
                product: Sku(1),
                quantity: 1,
                price: Money(2500),
            },
            Line {
                product: Sku(2),
                quantity: 2,
                price: Money(800),
            },
        ],
    };
    assert_eq!(order.total(), Money(4100));

    for products in [[].as_slice(), &catalog] {
        assert!(matches!(
            order.hold(&[], products),
            Some(Hold::AwaitingPayment {
                order: OrderNumber(7)
            })
        ));
    }

    let payments = [Payment {
        order: OrderNumber(7),
        amount: Money(4100),
        reference: String::from("TX-2041"),
    }];
    assert!(matches!(
        order.hold(&payments, &catalog),
        Some(Hold::OutOfStock {
            product: Sku(2),
            ..
        })
    ));

    catalog[1].stocked = 2;
    assert!(order.hold(&payments, &catalog).is_none());
}
