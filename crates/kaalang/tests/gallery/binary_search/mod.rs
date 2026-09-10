use std::cmp::Ordering;

use kaalang::kaalang;

#[kaalang]
fn binary_search(values: &[i32], target: i32) -> Option<usize> {
    #[action("Initialize the search range.")]
    let (mut left, mut right) = |values| (0usize, values.len());

    #[question("Does the search range contain any elements?")]
    while (|&left, &right| left < right) {
        #[action("Find the middle index.")]
        let mid = |left, right| left + (right - left) / 2;

        #[choice("Compare the middle element with the target.")]
        #[case("Search the right half.")]
        #[case("Search the left half.")]
        #[case("The target was found.")]
        let (less, greater, equal) = |values, target, mid| match values[mid].cmp(&target) {
            Ordering::Less => (),
            Ordering::Greater => (),
            Ordering::Equal => (),
        };

        #[action("Advance the lower bound.")]
        |less, mid, &mut left| *left = mid + 1;

        #[action("Reduce the upper bound.")]
        |greater, mid, &mut right| *right = mid;

        #[action("Return the matching index.")]
        let result = |equal, mid| Some(mid);
    }

    #[action("The target is absent.")]
    let result = || None;
}

#[kaalang]
fn binary_search_swapped(values: &[i32], target: i32) -> Option<usize> {
    #[action("Initialize the search range.")]
    let (mut left, mut right) = |values| (0usize, values.len());

    #[question("Does the search range contain any elements?")]
    #[no]
    #[yes]
    while (|&left, &right| left < right) {
        #[action("Find the middle index.")]
        let mid = |left, right| left + (right - left) / 2;

        #[choice("Compare the middle element with the target.")]
        #[case("The target was found.")]
        #[case("Search the right half.")]
        #[case("Search the left half.")]
        let (equal, less, greater) = |values, target, mid| match values[mid].cmp(&target) {
            Ordering::Equal => (),
            Ordering::Less => (),
            Ordering::Greater => (),
        };

        #[action("Return the matching index.")]
        let result = |equal, mid| Some(mid);

        #[action("Advance the lower bound.")]
        |less, mid, &mut left| *left = mid + 1;

        #[action("Reduce the upper bound.")]
        |greater, mid, &mut right| *right = mid;
    }

    #[action("The target is absent.")]
    let result = || None;
}

#[test]
fn finds_a_matching_index_or_reports_absence() {
    for values in [
        &[][..],
        &[5][..],
        &[-4, -1, 0, 3, 7][..],
        &[1, 1, 1, 3, 3][..],
    ] {
        for target in -6..=9 {
            for found in [
                binary_search(values, target),
                binary_search_swapped(values, target),
            ] {
                assert_eq!(found.is_some(), values.binary_search(&target).is_ok());
                if let Some(index) = found {
                    assert_eq!(values[index], target);
                }
            }
        }
    }
}
