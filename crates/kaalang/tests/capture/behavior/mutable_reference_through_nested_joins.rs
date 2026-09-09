use kaalang::kaalang;

#[kaalang]
fn mutable_reference_through_nested_joins(
    outer: bool,
    inner: bool,
    mut text: String,
) -> (String, usize) {
    #[question("Use the nested branch?")]
    |outer| -> (nested, fallback) { outer };

    #[question("Choose the inner mutation.")]
    |nested, inner| -> (first, second) { inner };

    #[action("Borrow and append the first suffix.")]
    |first, &mut text| -> partial {
        text.push('a');
        text
    };

    #[action("Borrow and append the second suffix.")]
    |second, &mut text| -> partial {
        text.push('b');
        text
    };

    #[action("Carry the borrowed value onward.")]
    |partial| -> view { partial };

    #[action("Borrow on the fallback branch.")]
    |fallback, &mut text| -> view { text };

    #[action("Mutate through the reference after both joins.")]
    |view| -> length {
        view.push('!');
        view.len()
    };

    #[action("Move the owner after the reference's last use.")]
    |text, length| -> result { (text, length) };
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
