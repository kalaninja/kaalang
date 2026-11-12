use kaalang::kaa;

/// A `Copy` wire is taken by value and then borrowed further down. kaalang
/// fixes the order; Rust decides that the first capture copies.
#[kaa]
fn copy_then_borrow(value: u32) -> u32 {
    #[kaa::action("Copy the value and increment it.")]
    let next = |value| value + 1;

    #[kaa::action("Log the original value.")]
    |&value| {
        assert_eq!(*value, 1);
    };

    |next| return next;
}

#[test]
fn a_copy_wire_survives_its_first_capture() {
    assert_eq!(copy_then_borrow(1), 2);
}
