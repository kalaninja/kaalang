use kaalang::kaalang;

#[kaalang]
fn mutate_wires(mut r#type: String, count: u32) -> (String, u32, usize) {
    #[action("Increment a local copy.")]
    let incremented = |mut count| {
        count += 1;
        count
    };

    #[action("Append to the original text.")]
    |&mut r#type| {
        r#type.push('!');
    };

    #[action("Measure the changed text.")]
    let length = |&r#type| r#type.len();

    #[action("Move the text and keep the original count.")]
    let (mut text, mut total) = |mut r#type, count, incremented| {
        r#type.push('?');
        (r#type, count + incremented)
    };

    #[action("Change both action outputs.")]
    |&mut text, &mut total| {
        text.push('.');
        *total += 10;
    };

    #[action("Return the changed wires.")]
    let end = |text, total, length| (text, total, length);
}

#[test]
fn mutable_borrows_change_wires_but_mutable_copies_do_not() {
    assert_eq!(mutate_wires("a".into(), 1), ("a!?.".into(), 13, 2));
}
