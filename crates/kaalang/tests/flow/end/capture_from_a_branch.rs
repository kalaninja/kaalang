use std::{cell::Cell, rc::Rc};

use kaalang::kaa;

struct Guard(Rc<Cell<bool>>);
impl Drop for Guard {
    fn drop(&mut self) {
        self.0.set(true);
    }
}

#[kaa]
fn capture_from_a_branch(condition: bool, dropped: Rc<Cell<bool>>) -> Rc<Cell<bool>> {
    #[kaa::action("Create a root return guard.")]
    let guard = |&dropped| Guard(dropped.clone());

    #[kaa::question("Is the short answer enough?")]
    let (end, more) = |condition| condition;

    #[kaa::action("Finish the longer branch.")]
    let result = |more, guard, dropped| dropped;

    #[kaa::action("Finish the short branch.")]
    let result = |end, guard, dropped| dropped;

    |result| return result;
}

#[test]
fn a_root_return_drops_its_unused_capture_before_the_caller_continues() {
    for condition in [true, false] {
        let dropped = Rc::new(Cell::new(false));
        let returned = capture_from_a_branch(condition, dropped);
        assert!(returned.get());
    }
}
