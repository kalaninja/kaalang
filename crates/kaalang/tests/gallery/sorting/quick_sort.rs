//! In-place quicksort. Recurse on the smaller partition and iterate over the
//! larger one, keeping the call stack logarithmic without an explicit stack of ranges.

use core::cmp::Ordering;

use kaalang::kaalang;

#[kaalang]
fn quick_sort<T: Ord>(mut values: &mut [T]) {
    #[action("Start with the whole slice.")]
    let (mut start, mut end) = |&values| (0, values.len());

    #[cycle("Sort the remaining range.")]
    let sorted = {
        #[question("Does the range contain at least two values?")]
        let (split, sorted) = |start, end| end - start > 1;

        #[action("Take the current range of the original slice.")]
        let mut range = |split, start, end, &mut values| &mut values[start..end];

        #[call("Partition into values below, equal to, and above the pivot.")]
        let (lower, upper) = |&mut range| partition(range);

        #[action("Take the smaller side for recursion; keep the larger range for later.")]
        let (smaller, remaining) = |range, start, end, lower, upper| {
            if lower <= range.len() - upper {
                (&mut range[..lower], (start + upper, end))
            } else {
                (&mut range[upper..], (start, start + lower))
            }
        };

        #[call("Recursively sort the smaller range.")]
        let sorted_part = |smaller| quick_sort(smaller);

        #[action("Continue with the larger range.")]
        |sorted_part, remaining, &mut start, &mut end| {
            (*start, *end) = remaining;
        };

        |sorted_part| continue;
    };

    |sorted| return;
}

#[test]
fn sorts_empty_duplicate_ordered_and_owned_values() {
    for mut input in [
        vec![],
        vec![4],
        vec![3, 1, 3, 2, 1],
        vec![1, 2, 3],
        vec![3, 2, 1],
        vec![7; 256],
        (0..1024).collect(),
        (0..1024).rev().collect(),
    ] {
        let mut expected = input.clone();
        expected.sort_unstable();
        quick_sort(&mut input);
        assert_eq!(input, expected);
    }
    let mut owned = [String::from("z"), String::from("a"), String::from("m")];
    let mut expected = owned.clone();
    expected.sort();
    quick_sort(&mut owned);
    assert_eq!(owned, expected);

    let mut partial = [99, 4, 1, 3, 2, -99];
    quick_sort(&mut partial[1..5]);
    assert_eq!(partial, [99, 1, 2, 3, 4, -99]);
}

#[test]
fn sorts_all_short_ternary_sequences() {
    for length in 0..=7 {
        for mut encoded in 0..3usize.pow(length) {
            let mut input = (0..length)
                .map(|_| {
                    let value = encoded % 3;
                    encoded /= 3;
                    value
                })
                .collect::<Vec<_>>();
            let mut expected = input.clone();
            expected.sort_unstable();
            quick_sort(&mut input);
            assert_eq!(input, expected);
        }
    }
}

/// Returns the half-open bounds of the equal region in a nonempty slice.
#[kaalang]
fn partition<T: Ord>(mut values: &mut [T]) -> (usize, usize) {
    #[action("Move the middle pivot to the end; start the three regions.")]
    let (mut lower, mut cursor, mut upper, pivot) = |&mut values| {
        assert!(!values.is_empty());
        let pivot = values.len() - 1;
        values.swap(values.len() / 2, pivot);
        (0, 0, pivot, pivot)
    };

    #[cycle("Classify every value around the pivot.")]
    let classified = {
        #[question("Does an unclassified value remain?")]
        let (compare, classified) = |cursor, upper| cursor < upper;

        #[choice("Where does this value belong?")]
        #[case("Less than the pivot.")]
        #[case("Equal to the pivot.")]
        #[case("Greater than the pivot.")]
        let (less, equal, greater) =
            |compare, &values, cursor, pivot| match values[cursor].cmp(&values[pivot]) {
                Ordering::Less => (),
                Ordering::Equal => (),
                Ordering::Greater => (),
            };

        #[action("Move the value left; advance lower and cursor.")]
        let stepped = |less, &mut values, &mut lower, &mut cursor| {
            values.swap(*cursor, *lower);
            *lower += 1;
            *cursor += 1;
        };

        #[action("Keep the equal value; advance cursor.")]
        let stepped = |equal, &mut cursor| *cursor += 1;

        #[action("Move the value right; inspect its replacement next.")]
        let stepped = |greater, &mut values, cursor, &mut upper| {
            *upper -= 1;
            values.swap(cursor, *upper);
        };

        |stepped| continue;
    };

    #[action("Place the pivot between the equal and greater regions.")]
    let bounds = |classified, &mut values, lower, upper, pivot| {
        values.swap(upper, pivot);
        (lower, upper + 1)
    };

    |bounds| return bounds;
}

#[test]
fn separates_three_regions_without_losing_values() {
    for mut input in [
        vec![4],
        vec![3, 1, 3, 2, 1],
        vec![1, 2, 3],
        vec![3, 2, 1],
        vec![7; 256],
    ] {
        let pivot = input[input.len() / 2];
        let mut expected = input.clone();
        expected.sort_unstable();
        let (lower, upper) = partition(&mut input);
        assert!(lower < upper);
        assert!(input[..lower].iter().all(|value| *value < pivot));
        assert!(input[lower..upper].iter().all(|value| *value == pivot));
        assert!(input[upper..].iter().all(|value| *value > pivot));
        input.sort_unstable();
        assert_eq!(input, expected);
    }
}
