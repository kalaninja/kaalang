use std::future::Future;
use std::rc::Rc;
use std::task::{Context, Poll, Waker};

use kaalang::kaalang;

#[kaalang]
async fn send_future_with_alternative_producers(
    condition: bool,
    first: bool,
    second: bool,
    third: bool,
) -> u8 {
    #[action("Wait before producing any values.")]
    || -> ready { std::future::ready(()).await };

    #[question("Choose a value.")]
    |ready, condition| -> (yes, no) { condition };

    #[action("Build the yes value and a local marker.")]
    |yes| -> (result, _marker) { (1u8, Rc::new(3u8)) };

    #[action("Wait again only on the no branch.")]
    |no| -> waited { std::future::ready(()).await };

    #[action("Build the no value and a local marker.")]
    |waited| -> (result, _marker) { (2u8, Rc::new(4u8)) };

    #[question("Enable the first independent input.")]
    |first| -> (a, _first_no) { first };

    #[question("Enable the second independent input.")]
    |second| -> (b, _second_no) { second };

    #[question("Enable the third independent input.")]
    |third| -> (c, _third_no) { third };

    #[action("Use the first and second inputs.")]
    |&a, &b| -> () {};

    #[action("Use the second and third inputs.")]
    |&b, &c| -> () {};

    #[action("Use the first and third inputs.")]
    |&a, &c| -> () {};

    #[end]
    |result| {};
}

#[test]
fn type_gates_do_not_carry_producer_auto_traits_across_awaits() {
    fn poll_once(future: impl Future<Output = u8> + Send) -> Poll<u8> {
        std::pin::pin!(future).poll(&mut Context::from_waker(Waker::noop()))
    }

    for (condition, expected) in [(true, 1), (false, 2)] {
        for selection in 0..8 {
            assert_eq!(
                poll_once(send_future_with_alternative_producers(
                    condition,
                    selection & 1 != 0,
                    selection & 2 != 0,
                    selection & 4 != 0,
                )),
                Poll::Ready(expected),
            );
        }
    }
}
