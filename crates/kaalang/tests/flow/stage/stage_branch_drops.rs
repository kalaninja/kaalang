use std::cell::RefCell;

use kaalang::kaa;

struct Guard<'a>(&'a RefCell<Vec<&'static str>>, &'static str);

impl Drop for Guard<'_> {
    fn drop(&mut self) {
        self.0.borrow_mut().push(self.1);
    }
}

#[kaa]
fn stage_branch_drops(choose: bool, events: &RefCell<Vec<&'static str>>) {
    #[kaa::stage("Choose a branch.")]
    let finish = |choose| {
        #[kaa::question("Take the left branch?")]
        let (left, right) = |choose| choose;

        #[kaa::action("Own the left branch guard.")]
        let left_guard = |left, events| Guard(events, "left");

        #[kaa::action("Finish the left branch.")]
        let finish = |&left_guard| {};

        #[kaa::action("Own the right branch guard.")]
        let right_guard = |right, events| Guard(events, "right");

        #[kaa::action("Finish the right branch.")]
        let finish = |&right_guard| {};
    };

    #[kaa::stage("Observe the next visit.")]
    |finish| {
        #[kaa::action("Record the visit.")]
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
