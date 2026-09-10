use kaalang::kaalang;

#[kaalang]
fn first_value(values: &[i32]) -> Option<i32> {
    #[question("Is there a first value?")]
    while (|values| !values.is_empty()) {
        #[action("Return the first value.")]
        let result = |values| Some(values[0]);
    }
    #[action("No first value exists.")]
    let result = || None;
}

#[test]
fn a_body_that_always_finishes_has_no_repeat() {
    assert_eq!(first_value(&[]), None);
    assert_eq!(first_value(&[7, 8]), Some(7));
}
