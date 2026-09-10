use std::cmp::Ordering;

use kaalang::kaalang;

#[kaalang]
fn binary_search(values: &[i32], target: i32) -> Option<usize> {
    #[action("Initialize the search range.")]
    let (mut left, mut right) = |values| (0, values.len());

    #[question("Does the search range contain any elements?")]
    while (|&left, &right| left < right) {
        #[action("Find the middle index.")]
        let mid = |left, right| left + (right - left) / 2;

        #[choice("Compare the middle element with the target.")]
        #[case("Less than the target.")]
        #[case("Greater than the target.")]
        #[case("Equal to the target.")]
        let (less, greater, equal) = |values, target, mid| match values[mid].cmp(&target) {
            Ordering::Less => (),
            Ordering::Greater => (),
            Ordering::Equal => (),
        };

        #[action("Search the right half.")]
        |less, mid, &mut left| *left = mid + 1;

        #[action("Search the left half.")]
        |greater, mid, &mut right| *right = mid;

        #[action("The target was found.")]
        let end = |equal, mid| Some(mid);
    }

    #[action("The target is absent.")]
    let end = || None;
}

#[kaalang]
fn binary_search_swapped(values: &[i32], target: i32) -> Option<usize> {
    #[action("Initialize the search range.")]
    let (mut left, mut right) = |values| (0, values.len());

    #[question("Does the search range contain any elements?")]
    #[no]
    #[yes]
    while (|&left, &right| left < right) {
        #[action("Find the middle index.")]
        let mid = |left, right| left + (right - left) / 2;

        #[choice("Compare the middle element with the target.")]
        #[case("Equal to the target.")]
        #[case("Less than the target.")]
        #[case("Greater than the target.")]
        let (equal, less, greater) = |values, target, mid| match values[mid].cmp(&target) {
            Ordering::Equal => (),
            Ordering::Less => (),
            Ordering::Greater => (),
        };

        #[action("The target was found.")]
        let end = |equal, mid| Some(mid);

        #[action("Search the right half.")]
        |less, mid, &mut left| *left = mid + 1;

        #[action("Search the left half.")]
        |greater, mid, &mut right| *right = mid;
    }

    #[action("The target is absent.")]
    let end = || None;
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
