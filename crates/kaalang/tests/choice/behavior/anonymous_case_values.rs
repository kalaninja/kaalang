use std::{
    pin::pin,
    task::{Context, Poll, Waker},
};

use kaalang::kaalang;

#[kaalang]
fn anonymous_case_values(value: i32) -> i32 {
    #[choice("Which transformation?")]
    #[case("Negate the value.")]
    #[case("Double the value.")]
    |value| -> (negate, double) {
        match value {
            ..0 => move || -value,
            _ => move || value * 2,
        }
    };

    #[action("Run the negation.")]
    |negate| -> selected { negate() };

    #[action("Run the doubling.")]
    |double| -> selected { double() };

    #[action("Capture the selected value in an async block.")]
    |selected| -> pending { async move { selected + 1 } };

    #[action("Resolve the async block.")]
    |pending| -> result {
        let pending = pin!(pending);
        let Poll::Ready(result) = pending.poll(&mut Context::from_waker(Waker::noop())) else {
            unreachable!("an async block without await points resolves on its first poll")
        };
        result
    };
}

#[test]
fn case_values_of_distinct_anonymous_types_converge_once() {
    assert_eq!(anonymous_case_values(-4), 5);
    assert_eq!(anonymous_case_values(3), 7);
}
