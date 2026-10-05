use kaalang::kaalang;

#[kaalang]
fn for_in_stage(values: &[u32]) -> u32 {
    #[action("Start with a total of zero.")]
    let mut total = || 0;

    #[action("Enter the stage.")]
    let go = || {};

    #[stage("Add every value, then return the total.")]
    |go| {
        #[cycle("Add every value.")]
        let added = |values| {
            for value in values {
                #[action("Add the value.")]
                |value, &mut total| *total += *value;
            }
        };

        |added, total| return total;
    };
}

#[test]
fn a_for_cycle_completes_inside_a_stage() {
    assert_eq!(for_in_stage(&[]), 0);
    assert_eq!(for_in_stage(&[1, 2, 3]), 6);
}
