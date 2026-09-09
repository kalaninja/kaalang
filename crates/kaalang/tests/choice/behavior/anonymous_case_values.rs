use kaalang::kaalang;

#[kaalang]
fn anonymous_case_values(value: i32) -> i32 {
    #[choice("Which transformation?")]
    #[case("Negate the value.")]
    #[case("Double the value.")]
    let (negate, double) = |value| match value {
        ..0 => move || -value,
        _ => move || value * 2,
    };

    #[action("Run the negation.")]
    let selected = |negate| negate();

    #[action("Run the doubling.")]
    let selected = |double| double();

    #[action("Capture the selected value in a closure.")]
    let increment = |selected| move || selected + 1;

    #[action("Run the selected closure.")]
    let result = |increment| increment();
}

#[test]
fn case_values_of_distinct_anonymous_types_converge_once() {
    assert_eq!(anonymous_case_values(-4), 5);
    assert_eq!(anonymous_case_values(3), 7);
}
