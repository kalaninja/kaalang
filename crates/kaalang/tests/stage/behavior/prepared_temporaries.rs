use std::cell::RefCell;

use kaalang::kaalang;

struct Guard<'a>(&'a RefCell<Vec<&'static str>>);

impl Drop for Guard<'_> {
    fn drop(&mut self) {
        self.0.borrow_mut().push("drop");
    }
}

#[kaalang]
fn prepared_temporaries(events: &RefCell<Vec<&'static str>>) -> (usize, usize) {
    #[action("Borrow temporary shared data, an entry value, and a guard.")]
    let (_guard, view, go) = |events| {
        (
            &Guard(events),
            &String::from("shared"),
            &String::from("entry"),
        )
    };

    #[stage("Use the temporary owners while the guard remains alive.")]
    |go| {
        #[action("Record the visit and measure both values.")]
        let lengths = |view, go, events| {
            events.borrow_mut().push("visit");
            (view.len(), go.len())
        };

        |lengths| return lengths;
    };
}

#[test]
fn preparation_preserves_temporary_lifetimes_and_drop_order() {
    let events = RefCell::new(Vec::new());
    assert_eq!(prepared_temporaries(&events), (6, 5));
    assert_eq!(*events.borrow(), ["visit", "drop"]);
}
