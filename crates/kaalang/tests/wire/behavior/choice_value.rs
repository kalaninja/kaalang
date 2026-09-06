use kaalang::kaalang;

#[kaalang]
fn choice_value(case: u8) -> &'static str {
    #[choice("Choose one of three values.")]
    #[case("First")]
    #[case("Second")]
    #[case("Third")]
    |case| -> (first, second, third) {
        match case {
            0 => (),
            1 => (),
            _ => (),
        }
    };

    #[action("Build the first value.")]
    |first| -> selected { "first" };

    #[action("Build the second value.")]
    |second| -> selected { "second" };

    #[action("Build the third value.")]
    |third| -> selected { "third" };

    #[action("Use the selected value.")]
    |selected| -> result { selected };
}

#[test]
fn choice_converges_three_producers() {
    assert_eq!(choice_value(0), "first");
    assert_eq!(choice_value(1), "second");
    assert_eq!(choice_value(2), "third");
}
