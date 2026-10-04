use kaalang::kaalang;

#[kaalang]
fn for_with_a_diverging_branch(values: &[u32]) -> u32 {
    #[action("Start with a total of zero.")]
    let mut total = || 0;

    #[cycle("Add the values, spinning forever on a zero.")]
    |values| {
        for value in values {
            #[question("Is the value zero?")]
            let (zero, nonzero) = |value| *value == 0;

            #[action("Add the value.")]
            |nonzero, value, &mut total| *total += *value;

            #[cycle("Spin forever.")]
            |zero| loop {
                #[action("Wait.")]
                let waited = || std::hint::spin_loop();

                |waited| continue;
            };
        }
    };

    |total| return total;
}

/// Only the `nonzero` route ends the iteration; the `zero` route diverges and
/// has nothing to merge with.
#[test]
fn a_diverging_branch_need_not_merge_before_the_iteration_ends() {
    assert_eq!(for_with_a_diverging_branch(&[]), 0);
    assert_eq!(for_with_a_diverging_branch(&[1, 2, 3]), 6);
}
