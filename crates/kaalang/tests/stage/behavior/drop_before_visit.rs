use std::cell::RefCell;

use kaalang::kaalang;

struct OnDrop<'a>(&'a RefCell<Vec<&'static str>>);

impl Drop for OnDrop<'_> {
    fn drop(&mut self) {
        self.0.borrow_mut().push("drop");
    }
}

#[kaalang]
fn drop_before_visit(events: &RefCell<Vec<&'static str>>) {
    #[action("Begin the first visit.")]
    let first = || {};

    #[stage("Own a local value.")]
    let second = |first| {
        #[action("Create a stage local owner.")]
        let _local = |events| OnDrop(events);

        #[action("Continue to the second stage.")]
        let second = || {};
    };

    #[stage("Observe the next visit.")]
    |second| {
        #[action("Record entry after the local was dropped.")]
        |events| events.borrow_mut().push("next");

        return;
    };
}

#[test]
fn local_owner_drops_before_the_next_stage() {
    let events = RefCell::new(Vec::new());
    drop_before_visit(&events);
    assert_eq!(*events.borrow(), ["drop", "next"]);
}
