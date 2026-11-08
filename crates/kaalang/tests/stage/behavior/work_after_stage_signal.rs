use std::cell::RefCell;

use kaalang::kaa;
#[kaa]
fn work_after_stage_signal(go: (), events: &RefCell<Vec<&'static str>>) -> u8 {
    #[kaa::stage("Prepare the result.")]
    let finish = |go| {
        #[kaa::action("Create the outgoing signal.")]
        let finish = |go| 7u8;
        #[kaa::action("Run the remaining work.")]
        |events| events.borrow_mut().push("work");
    };
    #[kaa::stage("Return the result.")]
    |finish| {
        |finish| return finish;
    };
}
#[test]
fn work_after_a_signal_runs_before_its_transition() {
    let events = RefCell::new(Vec::new());
    assert_eq!(work_after_stage_signal((), &events), 7);
    assert_eq!(*events.borrow(), ["work"]);
}
