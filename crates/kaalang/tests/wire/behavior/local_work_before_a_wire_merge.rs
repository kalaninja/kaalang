use std::cell::Cell;

use kaalang::kaalang;

#[kaalang]
fn local_work_before_a_wire_merge(condition: bool, order: &Cell<u8>) -> u32 {
    #[question("Choose a value.")]
    |condition| -> (yes, no) { condition };

    #[action("Build the yes value and local note.")]
    |yes| -> (selected, local_note) { (21, 1u8) };

    #[action("Build the no value, which has no note to record.")]
    |no| -> (selected, noted) { (34, ()) };

    #[action("Use the merged value.")]
    |selected, &order| -> used {
        order.set(order.get() * 10 + 2);
        selected * 2
    };

    // Authored after its consumer on purpose: only the merge before `selected`
    // orders this block ahead of the block above, so the recorded order proves
    // the rule rather than the authored sequence.
    #[action("Record the note before leaving the yes branch.")]
    |local_note, &order| -> noted { order.set(order.get() * 10 + local_note) };

    #[action("Finish once the merged value and the note are both in.")]
    |used, noted| -> result { used };
}

#[test]
fn local_work_finishes_before_the_merged_value_is_captured() {
    for (condition, expected, expected_order) in [(true, 42, 12), (false, 68, 2)] {
        let order = Cell::new(0);
        assert_eq!(local_work_before_a_wire_merge(condition, &order), expected);
        assert_eq!(order.get(), expected_order);
    }
}
