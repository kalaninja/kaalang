use std::cell::RefCell;

use kaalang::kaalang;

struct Guard<'a>(&'a RefCell<Vec<&'static str>>);

impl Drop for Guard<'_> {
    fn drop(&mut self) {
        self.0.borrow_mut().push("drop");
    }
}

#[kaalang]
fn prepared_owner(events: &RefCell<Vec<&'static str>>) -> usize {
    #[action("Keep the guard until the flow completes.")]
    let _guard = |events| Guard(events);

    #[action("Own the prepared text.")]
    let owner = || String::from("prepared");

    #[action("Borrow the text and enter the stage.")]
    let (view, go) = |&owner| (owner.as_str(), ());

    #[stage("Use the prepared reference while the guard remains alive.")]
    |go| {
        #[action("Record the visit and measure the text.")]
        let length = |view, events| {
            events.borrow_mut().push("visit");
            view.len()
        };

        |length| return length;
    };
}

#[test]
fn preparation_keeps_uncaptured_and_indirectly_borrowed_owners_alive() {
    let events = RefCell::new(Vec::new());
    assert_eq!(prepared_owner(&events), 8);
    assert_eq!(*events.borrow(), ["visit", "drop"]);
}
