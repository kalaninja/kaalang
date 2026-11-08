use std::cell::Cell;

use kaalang::kaa;

/// Actions with no outputs at flow entry, inside a branch, and directly above
/// the block producing `end`, in both spellings.
#[kaa]
fn effects_without_outputs(condition: bool, log: &Cell<u32>) -> u32 {
    #[kaa::action("Record the flow entry.")]
    |&log| {
        log.set(log.get() * 10 + 1);
    };

    #[kaa::question("Which way?")]
    let (yes, no) = |condition, &log| condition;

    #[kaa::action("Mark the yes branch.")]
    let marked = |yes, &log| log.set(log.get() * 10 + 2);

    #[kaa::action("Record the marked branch.")]
    let () = |&marked, &log| log.set(log.get() * 10 + 3);

    #[kaa::action("Take the yes branch.")]
    let taken = |marked, &log| log.set(log.get() * 10 + 4);

    #[kaa::action("Take the no branch.")]
    let taken = |no, &log| log.set(log.get() * 10 + 5);

    #[kaa::action("Record the step above the result.")]
    |&log| {
        log.set(log.get() * 10 + 6);
    };

    #[kaa::action("Finish.")]
    let end = |taken, &log| log.get();

    |end| return end;
}

#[test]
fn a_block_without_outputs_runs_where_it_is_written() {
    for (condition, expected) in [(true, 12346), (false, 156)] {
        let log = Cell::new(0);
        assert_eq!(effects_without_outputs(condition, &log), expected);
        assert_eq!(log.get(), expected);
    }
}
