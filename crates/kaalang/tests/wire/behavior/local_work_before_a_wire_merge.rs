use std::cell::Cell;

use kaalang::kaalang;

#[kaalang]
fn local_work_before_a_wire_merge(condition: bool, order: &Cell<u8>) -> u32 {
    #[question("Choose a value.")]
    let (yes, no) = |condition| condition;

    #[action("Build the yes value and local note.")]
    let (selected, local_note) = |yes| (21, 1);

    #[action("Build the no value, which has no note to record.")]
    let (selected, noted) = |no| (34, ());

    // The yes branch still owes this note to the `selected` merge, so it is
    // written above the block that captures the merged value.
    #[action("Record the note before leaving the yes branch.")]
    let noted = |local_note, &order| order.set(order.get() * 10 + local_note);

    #[action("Use the merged value.")]
    let used = |selected, &order| {
        order.set(order.get() * 10 + 2);
        selected * 2
    };

    #[action("Finish once the merged value and the note are both in.")]
    let end = |used, noted| used;
}

#[test]
fn local_work_finishes_before_the_merged_value_is_captured() {
    for (condition, expected, expected_order) in [(true, 42, 12), (false, 68, 2)] {
        let order = Cell::new(0);
        assert_eq!(local_work_before_a_wire_merge(condition, &order), expected);
        assert_eq!(order.get(), expected_order);
    }
}
