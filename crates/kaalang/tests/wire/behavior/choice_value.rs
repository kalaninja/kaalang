use kaalang::kaalang;

#[kaalang]
fn choice_value(case: u8) -> &'static str {
    #[choice("Choose one of three values.")]
    #[case("First")]
    #[case("Second")]
    #[case("Third")]
    let (first, second, third) = |case| match case {
        0 => (),
        1 => (),
        _ => (),
    };

    #[action("Build the first value.")]
    let selected = |first| "first";

    #[action("Build the second value.")]
    let selected = |second| "second";

    #[action("Build the third value.")]
    let selected = |third| "third";

    #[action("Use the selected value.")]
    let end = |selected| selected;
}

#[test]
fn choice_converges_three_producers() {
    assert_eq!(choice_value(0), "first");
    assert_eq!(choice_value(1), "second");
    assert_eq!(choice_value(2), "third");
}
