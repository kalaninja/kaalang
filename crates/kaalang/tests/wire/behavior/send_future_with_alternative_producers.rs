use std::future::Future;
use std::rc::Rc;
use std::task::{Context, Poll, Waker};

use kaalang::kaalang;

#[kaalang]
async fn send_future_with_alternative_producers(condition: bool) -> u8 {
    #[action("Wait before producing any values.")]
    || -> ready { std::future::ready(()).await };

    #[question("Choose a value.")]
    |ready, condition| -> (yes, no) { condition };

    #[action("Wait again only on the yes branch.")]
    |yes| -> waited { std::future::ready(()).await };

    #[action("Build the yes value and a local marker.")]
    |waited| -> (result, _marker) { (1u8, Rc::new(3u8)) };

    #[action("Build the no value and a local marker.")]
    |no| -> (result, _marker) { (2u8, Rc::new(4u8)) };
}

#[test]
fn type_gates_preserve_send_across_awaits() {
    fn poll_once(future: impl Future<Output = u8> + Send) -> Poll<u8> {
        std::pin::pin!(future).poll(&mut Context::from_waker(Waker::noop()))
    }

    for (condition, expected) in [(true, 1), (false, 2)] {
        assert_eq!(
            poll_once(send_future_with_alternative_producers(condition)),
            Poll::Ready(expected),
        );
    }
}
