use std::cell::RefCell;

use kaalang::kaalang;

struct Guard<'a>(&'a RefCell<Vec<&'static str>>, &'static str);

impl Drop for Guard<'_> {
    fn drop(&mut self) {
        self.0.borrow_mut().push(self.1);
    }
}

#[kaalang]
fn prepared_branch_drop(choose: bool, events: &RefCell<Vec<&'static str>>) {
    #[question("Choose a preparation branch.")]
    let (yes, no) = |choose| choose;

    #[action("Create the left branch guard.")]
    let left_guard = |yes, events| Guard(events, "left");

    #[action("Finish the left branch.")]
    let go = |&left_guard| {};

    #[action("Create the right branch guard.")]
    let right_guard = |no, events| Guard(events, "right");

    #[action("Finish the right branch.")]
    let go = |&right_guard| {};

    #[stage("Visit after the preparation branches merge.")]
    |go| {
        #[action("Record the stage visit.")]
        |events| events.borrow_mut().push("visit");

        return;
    };
}

#[test]
fn preparation_branch_owners_drop_before_the_stage_visit() {
    for (choose, branch) in [(true, "left"), (false, "right")] {
        let events = RefCell::new(Vec::new());
        prepared_branch_drop(choose, &events);
        assert_eq!(*events.borrow(), [branch, "visit"]);
    }
}
