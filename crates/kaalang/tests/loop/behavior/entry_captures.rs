use kaalang::kaalang;
use std::{cell::RefCell, rc::Rc};

struct Entry(Rc<RefCell<Vec<usize>>>);
impl Drop for Entry {
    fn drop(&mut self) {
        self.0.borrow_mut().push(99);
    }
}

#[kaalang]
fn entry_captures(entry: Entry, mut log: Rc<RefCell<Vec<usize>>>) -> usize {
    #[action("Initialize the counter.")]
    let (mut count, ticket) = || (0, ());

    |entry, mut ticket, &count, &mut log| loop {
        #[question("Has the counter reached three?")]
        let (done, again) = |&count| *count == 3;

        |done, mut ticket, &log, &mut count| break;

        #[action("Record and advance the counter.")]
        |again, &mut count, &log| {
            log.borrow_mut().push(*count);
            *count += 1;
        };
    };

    #[action("Return the counter.")]
    let end = |count| count;
}

#[test]
fn entry_captures_drop_once_before_the_body_and_do_not_reserve_its_borrows() {
    let log = Rc::new(RefCell::new(Vec::new()));
    assert_eq!(entry_captures(Entry(log.clone()), log.clone()), 3);
    assert_eq!(*log.borrow(), [99, 0, 1, 2]);
}
