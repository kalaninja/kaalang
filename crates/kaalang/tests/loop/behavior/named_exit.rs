use kaalang::kaalang;
use std::{cell::RefCell, rc::Rc};

struct Guard(Rc<RefCell<Vec<&'static str>>>, &'static str);
impl Drop for Guard {
    fn drop(&mut self) {
        self.0.borrow_mut().push(self.1);
    }
}

#[kaalang]
fn named_exit(limit: usize, log: Rc<RefCell<Vec<&'static str>>>) -> usize {
    #[action("Initialize the counter.")]
    let mut count = || 0;

    |&count| 'search: loop {
        #[action("Enter the outer scope.")]
        let _outer = |&log| Guard(log.clone(), "outer");

        loop {
            #[action("Enter the inner scope.")]
            let inner = |&log| Guard(log.clone(), "inner");

            #[question("Has the search finished?")]
            let (done, again) = |&count, limit| *count >= limit;

            |done, inner| break 'search;

            #[action("Advance the search.")]
            |again, &mut count| *count += 1;
        }
    };

    #[action("Continue after the search.")]
    let end = |count, &log| {
        log.borrow_mut().push("after");
        count
    };
}

#[test]
fn exiting_an_outer_loop_drops_inner_scopes_and_runs_its_continuation() {
    for limit in [0, 1, 3] {
        let log = Rc::new(RefCell::new(Vec::new()));
        assert_eq!(named_exit(limit, log.clone()), limit);
        let mut expected = vec!["inner"; limit + 1];
        expected.extend(["outer", "after"]);
        assert_eq!(*log.borrow(), expected);
    }
}
