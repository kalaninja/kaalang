use std::cell::Cell;

use kaalang::kaalang;

/// Actions with no outputs at flow entry, inside a branch, and directly above
/// the block producing `result`, in both spellings.
#[kaalang]
fn effects_without_outputs(condition: bool, log: &Cell<u32>) -> u32 {
    #[action("Record the flow entry.")]
    |&log| {
        log.set(log.get() * 10 + 1);
    };

    #[question("Which way?")]
    |condition, &log| -> (yes, no) { condition };

    #[action("Mark the yes branch.")]
    |yes, &log| -> marked { log.set(log.get() * 10 + 2) };

    #[action("Record the marked branch.")]
    |&marked, &log| -> () { log.set(log.get() * 10 + 3) };

    #[action("Take the yes branch.")]
    |marked, &log| -> taken { log.set(log.get() * 10 + 4) };

    #[action("Take the no branch.")]
    |no, &log| -> taken { log.set(log.get() * 10 + 5) };

    #[action("Record the step above the result.")]
    |&log| {
        log.set(log.get() * 10 + 6);
    };

    #[action("Finish.")]
    |taken, &log| -> result { log.get() };
}

#[test]
fn a_block_without_outputs_runs_where_it_is_written() {
    for (condition, expected) in [(true, 12346), (false, 156)] {
        let log = Cell::new(0);
        assert_eq!(effects_without_outputs(condition, &log), expected);
        assert_eq!(log.get(), expected);
    }
}
