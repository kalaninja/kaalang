use std::cell::RefCell;

use kaalang::kaalang;

struct Owner<'a>(&'a RefCell<Vec<&'static str>>);

impl Drop for Owner<'_> {
    fn drop(&mut self) {
        self.0.borrow_mut().push("owner");
    }
}

struct Borrowed<'a>(&'a Owner<'a>);

impl Drop for Borrowed<'_> {
    fn drop(&mut self) {
        self.0.0.borrow_mut().push("borrowed");
    }
}

#[kaalang]
fn prepared_drop_order(events: &RefCell<Vec<&'static str>>) -> bool {
    #[action("Create the owner first.")]
    let z = |events| Owner(events);

    #[action("Create a dependent owner second.")]
    let a = |&z| Borrowed(z);

    #[action("Enter the stage.")]
    let go = || {};

    #[stage("Read both prepared values.")]
    |go| {
        #[action("Check the retained reference.")]
        let same = |&z, &a| core::ptr::eq(z, a.0);

        |same| return same;
    };
}

#[test]
fn prepared_values_drop_in_reverse_source_order() {
    let events = RefCell::new(Vec::new());
    assert!(prepared_drop_order(&events));
    assert_eq!(*events.borrow(), ["borrowed", "owner"]);
}
