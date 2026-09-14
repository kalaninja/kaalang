use kaalang::kaalang;

#[kaalang]
fn mutable_reference_through_nested_joins(
    outer: bool,
    inner: bool,
    mut text: String,
) -> (String, usize) {
    #[question("Use the nested branch?")]
    let (nested, fallback) = |outer| outer;

    #[question("Choose the inner mutation.")]
    let (first, second) = |nested, inner| inner;

    #[action("Borrow and append the first suffix.")]
    let partial = |first, &mut text| {
        text.push('a');
        text
    };

    #[action("Borrow and append the second suffix.")]
    let partial = |second, &mut text| {
        text.push('b');
        text
    };

    #[action("Carry the borrowed value onward.")]
    let view = |partial| partial;

    #[action("Borrow on the fallback branch.")]
    let view = |fallback, &mut text| text;

    #[action("Mutate through the reference after both joins.")]
    let length = |view| {
        view.push('!');
        view.len()
    };

    |text, length| return (text, length);
}

#[test]
fn a_mutable_reference_crosses_joins_while_its_owner_stays_alive() {
    for (outer, inner, expected) in [(true, true, "a!"), (true, false, "b!"), (false, true, "!")] {
        assert_eq!(
            mutable_reference_through_nested_joins(outer, inner, String::new()),
            (expected.into(), expected.len())
        );
    }
}
