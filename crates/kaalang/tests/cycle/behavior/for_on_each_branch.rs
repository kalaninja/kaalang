use kaalang::kaalang;

#[kaalang]
fn for_on_each_branch(values: &[i64], twice: bool) -> i64 {
    #[action("Start with a total of zero.")]
    let mut total = || 0;

    #[question("Should every value count twice?")]
    let (double, single) = |twice| twice;

    #[cycle("Add every value twice.")]
    let counted = |double, values| {
        for value in values {
            #[action("Add the value twice.")]
            |value, &mut total| *total += 2 * *value;
        }
    };

    #[cycle("Add every value once.")]
    let counted = |single, values| {
        for value in values {
            #[action("Add the value.")]
            |value, &mut total| *total += *value;
        }
    };

    |counted, total| return total;
}

#[test]
fn the_outputs_of_two_branch_cycles_merge() {
    assert_eq!(for_on_each_branch(&[1, 2], true), 6);
    assert_eq!(for_on_each_branch(&[1, 2], false), 3);
    assert_eq!(for_on_each_branch(&[], true), 0);
}
