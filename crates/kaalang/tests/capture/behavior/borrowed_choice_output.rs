use kaalang::kaalang;

/// A case value may be borrowed rather than moved, like any other wire.
#[kaalang]
fn borrowed_choice_output(input: Option<u8>) -> u8 {
    #[choice("Was a value supplied?")]
    #[case("A value is available.")]
    #[case("No value is available.")]
    let (value, absent) = |input| match input {
        Some(value) => value,
        None => (),
    };

    #[action("Borrow the supplied value.")]
    let end = |&value| *value * 2;

    #[action("Produce the absent result.")]
    let end = |absent| 0;

    |end| return end;
}

#[test]
fn a_borrowed_case_value_stays_readable() {
    assert_eq!(borrowed_choice_output(Some(21)), 42);
    assert_eq!(borrowed_choice_output(None), 0);
}
