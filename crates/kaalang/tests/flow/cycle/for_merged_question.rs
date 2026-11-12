use kaalang::kaa;

#[kaa]
fn for_merged_question(values: &[u32]) -> u32 {
    #[kaa::action("Start with a total of zero.")]
    let mut total = || 0;

    #[kaa::cycle("Add the even values.")]
    |values| {
        for value in values {
            #[kaa::question("Is the value even?")]
            let (even, _counted) = |value| *value % 2 == 0;

            #[kaa::action("Add the value.")]
            let _counted = |even, value, &mut total| *total += *value;
        }
    };

    |total| return total;
}

#[test]
fn branches_in_a_for_body_merge_before_the_next_item() {
    assert_eq!(for_merged_question(&[]), 0);
    assert_eq!(for_merged_question(&[1, 2, 3, 4]), 6);
}
