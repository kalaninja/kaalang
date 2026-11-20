//! The binary: its types draw cards in the library's row.

use kaalang::kaa;
use shop::{Order, OrderNumber, ids::next_number};

#[kaa("The counter where orders are taken.")]
struct Counter {
    #[kaa::refer(Order, "The order taken last.")]
    last: OrderNumber,
}

/// A flow in the binary that calls a flow of the library.
#[kaa]
fn take_order(last: OrderNumber) -> OrderNumber {
    #[kaa::call("Number the new order.")]
    let number = |last| next_number(last);

    |number| return number;
}

fn main() {
    let counter = Counter {
        last: OrderNumber(41),
    };
    let number = take_order(counter.last);
    println!("order {}", number.0);
}
