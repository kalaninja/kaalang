use kaalang::kaalang;

#[kaalang]
const fn const_mutable_captures(mut value: u32) -> u32 {
    #[action("Increment a local copy.")]
    |mut value| -> incremented {
        value += 1;
        value
    };

    #[action("Add the copy to the original wire.")]
    |&mut value, incremented| {
        *value += incremented;
    };

    #[action("Return the changed original.")]
    |value| -> result { value };
}

#[test]
fn both_mutable_capture_forms_work_during_constant_evaluation() {
    const VALUE: u32 = const_mutable_captures(3);
    assert_eq!(VALUE, 7);
}
