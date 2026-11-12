use kaalang::kaa;

#[kaa]
fn first_value(values: &[i32]) -> Option<i32> {
    #[kaa::cycle("Find the first value.")]
    let selected = |values| loop {
        #[kaa::question("Is there a first value?")]
        #[kaa::yes("YES")]
        #[kaa::no("NO")]
        let (iterate_1, leave_1) = |values| !values.is_empty();

        #[kaa::action("Report that no first value exists.")]
        let selected = |leave_1| None;

        #[kaa::action("Produce the first value.")]
        let selected = |iterate_1, values| Some(values[0]);
    };

    |selected| return selected;
}

#[test]
fn a_body_that_always_finishes_has_no_repeat() {
    assert_eq!(first_value(&[]), None);
    assert_eq!(first_value(&[7, 8]), Some(7));
}
