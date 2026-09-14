use kaalang::kaalang;

#[kaalang]
fn first_value(values: &[i32]) -> Option<i32> {
    #[cycle("Find the first value.")]
    let result = |values| {
        #[question("Is there a first value?")]
        #[yes("YES")]
        #[no("NO")]
        let (iterate_1, leave_1) = |values| !values.is_empty();

        #[action("Report that no first value exists.")]
        let absent = |leave_1| None;

        |absent| break absent;

        #[action("Produce the first value.")]
        let found = |iterate_1, values| Some(values[0]);

        |found| break found;
    };

    |result| return result;
}

#[test]
fn a_body_that_always_finishes_has_no_repeat() {
    assert_eq!(first_value(&[]), None);
    assert_eq!(first_value(&[7, 8]), Some(7));
}
