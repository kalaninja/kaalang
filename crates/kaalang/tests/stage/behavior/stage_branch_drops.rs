use std::cell::RefCell;

use kaalang::kaalang;

struct Guard<'a>(&'a RefCell<Vec<&'static str>>, &'static str);

impl Drop for Guard<'_> {
    fn drop(&mut self) {
        self.0.borrow_mut().push(self.1);
    }
}

#[kaalang]
fn stage_branch_drops(choose: bool, events: &RefCell<Vec<&'static str>>) {
    #[stage("Choose a branch.")]
    let finish = |choose| {
        #[question("Take the left branch?")]
        let (left, right) = |choose| choose;

        #[action("Own the left branch guard.")]
        let left_guard = |left, events| Guard(events, "left");

        #[action("Finish the left branch.")]
        let finish = |&left_guard| {};

        #[action("Own the right branch guard.")]
        let right_guard = |right, events| Guard(events, "right");

        #[action("Finish the right branch.")]
        let finish = |&right_guard| {};
    };

    #[stage("Observe the next visit.")]
    |finish| {
        #[action("Record the visit.")]
        |events| events.borrow_mut().push("visit");

        return;
    };
}

#[test]
fn branch_owners_drop_before_the_next_stage_visit() {
    for (choose, branch) in [(true, "left"), (false, "right")] {
        let events = RefCell::new(Vec::new());
        stage_branch_drops(choose, &events);
        assert_eq!(*events.borrow(), [branch, "visit"]);
    }
}
