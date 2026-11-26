use std::sync::Mutex;

use kaalang::kaa;

#[kaa]
fn unread_lock_guard(counter: &Mutex<u32>) -> u32 {
    #[kaa::action("Lock the counter.")]
    let mut guard = |counter| counter.lock().expect("no holder panicked");

    #[kaa::action("Count one while the lock is held.")]
    let count = |&mut guard| {
        **guard += 1;
        **guard
    };

    #[kaa::action("Release the lock.")]
    let released = |guard| {};

    |released, count| return count;
}

#[test]
fn a_lock_guard_captured_unread_is_held_until_its_block() {
    assert_eq!(unread_lock_guard(&Mutex::new(6)), 7);
}
