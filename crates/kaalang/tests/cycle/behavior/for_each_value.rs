use kaalang::kaalang;

// rustfmt would brace the `for` closure body; the flow keeps the unbraced
// spelling, which kaalang reads as the same cycle.
#[kaalang]
#[rustfmt::skip]
fn for_each_value(values: &[i64]) -> i64 {
    #[action("Start with a total of zero.")]
    let mut total = || 0;

    #[cycle("Add every value to the total.")]
    |values| for value in values {
        #[action("Add the value.")]
        |value, &mut total| *total += *value;
    };

    |total| return total;
}

#[test]
fn a_for_cycle_runs_its_body_once_per_item() {
    assert_eq!(for_each_value(&[]), 0);
    assert_eq!(for_each_value(&[7]), 7);
    assert_eq!(for_each_value(&[1, 2, 3, 4]), 10);
}
