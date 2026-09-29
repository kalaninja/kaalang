use kaalang::kaalang;

#[kaalang]
fn split_case_between_merges(value: u8, low: bool) -> u8 {
    #[choice("Which source?")]
    #[case("The first source.")]
    #[case("The middle source.")]
    #[case("The last source.")]
    let (first, middle, last) = |value| match value {
        0 => (),
        1 => (),
        _ => (),
    };

    #[question("Does the middle source lean low?")]
    let (lower, upper) = |middle, low| low;

    #[action("Build the low value from the first source.")]
    let low_value = |first| 10;

    #[action("Build the low value from the middle source.")]
    let low_value = |lower| 20;

    #[action("Build the high value from the middle source.")]
    let high_value = |upper| 30;

    #[action("Build the high value from the last source.")]
    let high_value = |last| 40;

    #[action("Finish a low value.")]
    let end = |low_value| low_value + 1;

    #[action("Finish a high value.")]
    let end = |high_value| high_value + 2;

    |end| return end;
}

#[test]
fn one_case_can_feed_two_disjoint_merges() {
    assert_eq!(split_case_between_merges(0, true), 11);
    assert_eq!(split_case_between_merges(1, true), 21);
    assert_eq!(split_case_between_merges(1, false), 32);
    assert_eq!(split_case_between_merges(2, false), 42);
}
