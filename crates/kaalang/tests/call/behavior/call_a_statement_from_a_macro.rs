use kaalang::kaalang;

use std::sync::atomic::{AtomicUsize, Ordering};

static NOTICES: AtomicUsize = AtomicUsize::new(0);

fn record() {
    NOTICES.fetch_add(1, Ordering::Relaxed);
}

/// A flow written by another macro, whose whole call statement arrives wrapped
/// in the invisible group a metavariable substitutes through.
macro_rules! recording {
    ($name:ident, $application:expr) => {
        #[kaalang]
        fn $name() -> usize {
            #[call]
            $application;

            #[action("Report the notices.")]
            let end = || NOTICES.load(Ordering::Relaxed);

            |end| return end;
        }
    };
}

recording!(call_a_statement_from_a_macro, record());

#[test]
fn a_call_statement_may_arrive_through_a_macro_metavariable() {
    assert_eq!(call_a_statement_from_a_macro(), 1);
}
