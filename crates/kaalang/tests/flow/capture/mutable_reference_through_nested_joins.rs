use kaalang::kaa;

#[kaa]
fn mutable_reference_through_nested_joins(
    outer: bool,
    inner: bool,
    mut text: String,
) -> (String, usize) {
    #[kaa::question("Use the nested branch?")]
    let (nested, fallback) = |outer| outer;

    #[kaa::question("Choose the inner mutation.")]
    let (first, second) = |nested, inner| inner;

    #[kaa::action("Borrow and append the first suffix.")]
    let partial = |first, &mut text| {
        text.push('a');
        text
    };

    #[kaa::action("Borrow and append the second suffix.")]
    let partial = |second, &mut text| {
        text.push('b');
        text
    };

    #[kaa::action("Carry the borrowed value onward.")]
    let view = |partial| partial;

    #[kaa::action("Borrow on the fallback branch.")]
    let view = |fallback, &mut text| text;

    #[kaa::action("Mutate through the reference after both joins.")]
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
