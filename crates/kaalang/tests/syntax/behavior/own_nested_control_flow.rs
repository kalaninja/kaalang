use std::{
    pin::pin,
    task::{Context, Poll, Waker},
};

use kaalang::kaalang;

/// RFC 0001 §3: a `return` expression or `?` operator inside a nested closure,
/// async block, or item definition belongs to that Rust construct, not to the
/// kaalang block body around it.
#[kaalang]
fn own_nested_control_flow(input: u32) -> u32 {
    #[action("Combine values from constructs that own their control flow.")]
    |input| -> result {
        // Lowering binds the inputs ahead of the authored statements, so an item
        // written first in the body would still follow a statement and trip
        // `clippy::items_after_statements`. Its own block keeps it first.
        let halved = {
            fn halve_even(value: u32) -> u32 {
                if value % 2 == 1 {
                    return value;
                }
                value / 2
            }
            halve_even(input)
        };
        let successor = |text: &str| -> Option<u32> { Some(text.parse::<u32>().ok()? + 1) };
        let future = async {
            if input == 0 {
                return 0;
            }
            input
        };
        let future = pin!(future);
        let Poll::Ready(polled) = future.poll(&mut Context::from_waker(Waker::noop())) else {
            unreachable!("an async block without await points resolves on its first poll")
        };
        halved + successor("1").unwrap_or(0) + polled
    };

    #[end]
    |result| {};
}

#[test]
fn nested_constructs_keep_their_own_return_and_try() {
    assert_eq!(own_nested_control_flow(4), 8);
    assert_eq!(own_nested_control_flow(3), 8);
    assert_eq!(own_nested_control_flow(0), 2);
}
