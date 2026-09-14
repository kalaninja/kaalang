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
    let mut count = || 0;

    #[cycle("Count three entries.")]
    let final_count = |entry, mut count| {
        #[question("Has the counter reached three?")]
        let (done, again) = |&count| *count == 3;

        |done, count, entry| break count;

        #[action("Record and advance the counter.")]
        |again, &mut count, &entry| {
            entry.0.borrow_mut().push(*count);
            *count += 1;
        };
    };

    #[action("Record work after the cycle input drops.")]
    let result = |final_count, &mut log| {
        log.borrow_mut().push(100);
        final_count
    };

    |result| return result;
}

#[test]
fn a_cycle_input_is_evaluated_once_and_drops_before_the_continuation() {
    let log = Rc::new(RefCell::new(Vec::new()));
    assert_eq!(entry_captures(Entry(log.clone()), log.clone()), 3);
    assert_eq!(*log.borrow(), [0, 1, 2, 99, 100]);
}
