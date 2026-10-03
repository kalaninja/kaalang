use std::cell::RefCell;

use kaalang::kaalang;
#[kaalang]
fn work_after_stage_signal(go: (), events: &RefCell<Vec<&'static str>>) -> u8 {
    #[stage("Prepare the result.")]
    let finish = |go| {
        #[action("Create the outgoing signal.")]
        let finish = |go| 7u8;
        #[action("Run the remaining work.")]
        |events| events.borrow_mut().push("work");
    };
    #[stage("Return the result.")]
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
