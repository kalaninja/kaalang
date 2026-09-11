use kaalang::kaalang;

#[kaalang]
fn first_value(values: &[i32]) -> Option<i32> {
    loop {
        #[question("Is there a first value?")]
        #[yes("YES")]
        #[no("NO")]
        let (iterate_1, leave_1) = |values| !values.is_empty();

        |leave_1| break;

        #[action("Return the first value.")]
        let end = |iterate_1, values| Some(values[0]);
    }

    #[action("No first value exists.")]
    let end = || None;
}

#[test]
fn a_body_that_always_finishes_has_no_repeat() {
    assert_eq!(first_value(&[]), None);
    assert_eq!(first_value(&[7, 8]), Some(7));
}
