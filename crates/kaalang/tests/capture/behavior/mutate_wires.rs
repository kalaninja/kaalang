use kaalang::kaalang;

#[kaalang]
fn mutate_wires(mut r#type: String, count: u32) -> (String, u32, usize) {
    #[action("Increment a local copy.")]
    |mut count| -> incremented {
        count += 1;
        count
    };

    #[action("Append to the original text.")]
    |&mut r#type| {
        r#type.push('!');
    };

    #[action("Measure the changed text.")]
    |&r#type| -> length { r#type.len() };

    #[action("Move the text and keep the original count.")]
    |mut r#type, count, incremented| -> (text, total) {
        r#type.push('?');
        (r#type, count + incremented)
    };

    #[action("Change both action outputs.")]
    |&mut text, &mut total| {
        text.push('.');
        *total += 10;
    };

    #[action("Return the changed wires.")]
    |text, total, length| -> result { (text, total, length) };
}

#[test]
fn mutable_borrows_change_wires_but_mutable_copies_do_not() {
    assert_eq!(mutate_wires("a".into(), 1), ("a!?.".into(), 13, 2));
}
