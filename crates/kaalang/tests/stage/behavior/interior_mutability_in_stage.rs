use kaalang::kaalang;
use std::cell::Cell;
#[kaalang]
fn interior_mutability_in_stage(go: Cell<u8>) -> u8 {
    #[stage("Update the cell.")]
    let finish = |go| {
        #[action("Change the inner value.")]
        let finish = |go| {
            go.set(go.get() + 1);
            go.get()
        };
    };

    #[stage("Return the new value.")]
    |finish| {
        |finish| return finish;
    };
}
#[test]
fn an_immutable_entry_can_hold_interior_mutability() {
    assert_eq!(interior_mutability_in_stage(Cell::new(4)), 5);
}
