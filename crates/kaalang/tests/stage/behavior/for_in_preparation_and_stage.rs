use kaalang::kaalang;

#[kaalang]
fn for_in_preparation_and_stage(values: &[u32]) -> u32 {
    #[cycle("Check every value.")]
    |values| {
        for value in values {
            #[action("Check that the value is small.")]
            |value| assert!(*value < 1000);
        }
    };

    #[action("Start with a total of zero.")]
    let mut total = || 0;

    #[action("Enter the stage.")]
    let go = || {};

    #[stage("Add every value, then return the total.")]
    |go| {
        #[cycle("Add every value.")]
        |values| {
            for value in values {
                #[action("Add the value.")]
                |value, &mut total| *total += *value;
            }
        };

        |total| return total;
    };
}

/// Neither cycle names its completion, and the names the compiler gives them
/// do not collide across the preparation and the stage.
#[test]
fn unnamed_completions_stay_apart_across_parts() {
    assert_eq!(for_in_preparation_and_stage(&[]), 0);
    assert_eq!(for_in_preparation_and_stage(&[1, 2, 3]), 6);
}
