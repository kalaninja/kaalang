use std::sync::atomic::{AtomicUsize, Ordering};

use kaalang::kaalang;

static VISITS: AtomicUsize = AtomicUsize::new(0);

macro_rules! counting {
    ($name:ident, $body:block) => {
        #[kaalang]
        fn $name() -> usize {
            #[action("Count a visit.")]
            $body;

            #[action("Read the count.")]
            let count = || VISITS.load(Ordering::Relaxed);

            |count| return count;
        }
    };
}

counting!(block_from_a_macro, {
    VISITS.fetch_add(1, Ordering::Relaxed);
});

#[test]
fn a_bare_action_block_from_a_macro_runs() {
    assert_eq!(block_from_a_macro(), 1);
}
