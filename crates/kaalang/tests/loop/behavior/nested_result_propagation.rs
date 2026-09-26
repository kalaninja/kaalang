use kaalang::kaalang;
use std::{cell::RefCell, rc::Rc};

struct Guard(Rc<RefCell<Vec<&'static str>>>, &'static str);
impl Drop for Guard {
    fn drop(&mut self) {
        self.0.borrow_mut().push(self.1);
    }
}

#[kaalang]
fn nested_result_propagation(limit: usize, log: Rc<RefCell<Vec<&'static str>>>) -> usize {
    #[action("Initialize the counter.")]
    let mut count = || 0;

    #[cycle("Search until the inner cycle finds the limit.")]
    let next = {
        #[action("Enter the outer scope.")]
        let _outer = |&log| Guard(log.clone(), "outer");

        #[cycle("Advance until the search is done.")]
        let next = {
            #[action("Enter the inner scope.")]
            let inner = |&log| Guard(log.clone(), "inner");

            #[question("Has the search finished?")]
            let (done, again) = |&count, limit| *count >= limit;

            #[action("Hand over the count and the log.")]
            let next = |done, count, log, inner| (count, log);

            #[action("Advance the search.")]
            |again, &mut count| *count += 1;

            |again| continue;
        };
    };

    #[action("Continue after the search.")]
    let result = |next| {
        let (final_count, final_log) = next;
        final_log.borrow_mut().push("after");
        final_count
    };

    |result| return result;
}

#[test]
fn nested_results_drop_inner_scopes_and_run_the_outer_continuation() {
    for limit in [0, 1, 3] {
        let log = Rc::new(RefCell::new(Vec::new()));
        assert_eq!(nested_result_propagation(limit, log.clone()), limit);
        let mut expected = vec!["inner"; limit + 1];
        expected.extend(["outer", "after"]);
        assert_eq!(*log.borrow(), expected);
    }
}
