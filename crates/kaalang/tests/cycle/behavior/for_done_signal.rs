use kaalang::kaalang;

#[kaalang]
fn for_done_signal(values: &[i64]) -> i64 {
    #[action("Start with a total of zero.")]
    let mut total = || 0;

    #[cycle("Add every value to the total.")]
    let added = |values| {
        for value in values {
            #[action("Add the value.")]
            |value, &mut total| *total += *value;
        }
    };

    |added, total| return total;
}

#[test]
fn the_declared_output_signals_that_the_items_ran_out() {
    assert_eq!(for_done_signal(&[]), 0);
    assert_eq!(for_done_signal(&[5, -5, 5]), 5);
}
