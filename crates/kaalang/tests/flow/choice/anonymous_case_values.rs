use kaalang::kaa;

#[kaa]
fn anonymous_case_values(value: i32) -> i32 {
    #[kaa::choice("Which transformation?")]
    #[kaa::case("Negate the value.")]
    #[kaa::case("Double the value.")]
    let (negate, double) = |value| match value {
        ..0 => move || -value,
        _ => move || value * 2,
    };

    #[kaa::action("Run the negation.")]
    let selected = |negate| negate();

    #[kaa::action("Run the doubling.")]
    let selected = |double| double();

    #[kaa::action("Capture the selected value in a closure.")]
    let increment = |selected| move || selected + 1;

    #[kaa::action("Run the selected closure.")]
    let end = |increment| increment();

    |end| return end;
}

#[test]
fn case_values_of_distinct_anonymous_types_converge_once() {
    assert_eq!(anonymous_case_values(-4), 5);
    assert_eq!(anonymous_case_values(3), 7);
}
