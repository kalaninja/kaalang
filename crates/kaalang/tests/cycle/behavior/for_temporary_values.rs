use std::cell::RefCell;

use kaalang::kaalang;

thread_local! {
    static ORDER: RefCell<Vec<&'static str>> = const { RefCell::new(Vec::new()) };
}

struct Values([u32; 3]);

impl Values {
    fn new() -> Self {
        record("build");
        Self([1, 2, 3])
    }
}

impl Drop for Values {
    fn drop(&mut self) {
        record("drop");
    }
}

fn record(event: &'static str) {
    ORDER.with_borrow_mut(|order| order.push(event));
}

#[kaalang]
fn for_temporary_values() -> u32 {
    #[action("Start with a total of zero.")]
    let mut total = || 0;

    #[cycle("Add values borrowed from a temporary owner.")]
    for value in Values::new().0.iter() {
        #[action("Add the value.")]
        |value, &mut total| {
            record("visit");
            *total += *value;
        };
    }

    #[action("Finish after releasing the temporary owner.")]
    || record("finish");

    |total| return total;
}

#[test]
fn a_temporary_owner_is_built_once_and_drops_after_iteration() {
    ORDER.with_borrow_mut(Vec::clear);
    assert_eq!(for_temporary_values(), 6);
    ORDER.with_borrow(|order| {
        assert_eq!(
            order,
            &["build", "visit", "visit", "visit", "drop", "finish"]
        );
    });
}
