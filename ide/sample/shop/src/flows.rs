//! Flows of every shape. The diagram shows the first flow of the file until
//! the cursor enters one.

use kaalang::kaa;

use crate::{Hold, Order, catalog::Money};

/// The sum of the order's lines. Not a flow: a call node names it.
pub fn total(order: &Order) -> Money {
    Money(
        order
            .lines
            .iter()
            .map(|line| line.price.0 * u64::from(line.quantity))
            .sum(),
    )
}

/// Branches that merge, a call, and wires whose types have cards.
#[kaa]
pub fn review(order: &Order, paid: Money) -> Option<Hold> {
    #[kaa::call("Add up the order's lines.")]
    let due = |order| total(order);

    #[kaa::question("Did the customer pay the whole amount?")]
    #[kaa::yes("YES")]
    #[kaa::no("NO")]
    let (paid_up, short) = |due, paid| paid >= due;

    #[kaa::action("⏳ Hold the order until the rest of the money arrives.")]
    let hold = |short, order| Some(Hold::AwaitingPayment(order.number));

    #[kaa::choice("Does the order list any products?")]
    #[kaa::case("None.")]
    #[kaa::case("Some.")]
    let (empty, filled) = |paid_up, order| match order.lines.is_empty() {
        true => (),
        false => (),
    };

    #[kaa::action("📭 Nothing to pack, nothing to hold.")]
    let hold = |empty| None;

    #[kaa::action("🔎 Hold the order for a line nobody can pack.")]
    let hold = |filled, order| {
        let line = order.lines.iter().find(|line| line.quantity == 0);
        line.map(|line| Hold::OutOfStock {
            order: order.number,
            product: line.product,
        })
    };

    |hold| return hold;
}

/// A `for` cycle.
#[kaa]
pub fn count_items(order: &Order) -> u32 {
    #[kaa::action("🧮 Start with no items.")]
    let mut count = || 0;

    #[kaa::cycle("Add each line's quantity.")]
    let counted = |order| {
        for line in &order.lines {
            #[kaa::action("➕ Add this line's quantity.")]
            |line, &mut count| *count += line.quantity;
        }
    };

    |counted, count| return count;
}

/// A `loop` cycle that leaves through a question.
#[kaa]
pub fn boxes(items: u32, per_box: u32) -> u32 {
    #[kaa::action("📦 Nothing is packed yet.")]
    let (mut left, mut packed) = |items| (items, 0);

    #[kaa::cycle("Fill boxes until every item is in one.")]
    let boxes = loop {
        #[kaa::question("Are items left?")]
        #[kaa::yes("YES")]
        #[kaa::no("NO")]
        let (pack, leave) = |left| left > 0;

        #[kaa::action("🏁 Every item is in a box.")]
        let boxes = |leave, packed| packed;

        #[kaa::action("📦 Fill one more box.")]
        let filled = |pack, per_box, &mut left, &mut packed| {
            *left = left.saturating_sub(per_box.max(1));
            *packed += 1;
        };

        |filled| continue;
    };

    |boxes| return boxes;
}

/// Stages: the flow hands over from one to the next.
#[kaa]
pub fn restock(missing: u32) -> u32 {
    #[kaa::action("🗒️ Nothing ordered from the supplier yet.")]
    let mut ordered = || 0u32;

    #[kaa::action("Start ordering.")]
    let order = || {};

    #[kaa::stage("Order from the supplier until the shelf is full.")]
    let (order, done) = |order| {
        #[kaa::question("Is the shelf full?")]
        let (done, again) = |ordered, missing| ordered >= missing;

        #[kaa::action("🚚 Order one more unit.")]
        let order = |again, &mut ordered| {
            *ordered += 1;
        };
    };

    #[kaa::stage("Report what was ordered.")]
    |done| {
        |ordered| return ordered;
    };
}

impl Order {
    /// A method: its path is `Order::ship`, and its receiver is a wire. The
    /// `gift` feature is off, so `_wrapped` is no wire.
    #[kaa]
    pub fn ship(&self, #[cfg(feature = "gift")] _wrapped: bool, days: u32) -> &'static str {
        #[kaa::question("Can it arrive tomorrow?")]
        #[kaa::yes("YES")]
        #[kaa::no("NO")]
        let (fast, slow) = |&self, days| days <= 1 && !self.lines.is_empty();

        #[kaa::action("🚀 Tomorrow.")]
        let when = |fast| "tomorrow";

        #[kaa::action("🐢 This week.")]
        let when = |slow| "this week";

        |when| return when;
    }
}
