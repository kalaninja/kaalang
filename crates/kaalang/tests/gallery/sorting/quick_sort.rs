//! In-place three-way quicksort. Recursing on the smaller side and iterating
//! over the larger one keeps the call stack logarithmic.

use core::cmp::Ordering;

use kaalang::kaalang;

#[kaalang]
pub(crate) fn quick_sort<T: Ord>(values: &mut [T]) {
    #[stage("Check the range.")]
    let (partition, finish) = |values| {
        #[question("Are there at least two values?")]
        #[yes("YES")]
        #[no("NO")]
        let (split, finish) = |&values| values.len() > 1;

        #[action("Take the unsorted range.")]
        let partition = |split, values| values;
    };

    #[stage("Partition the range.")]
    let recur = |partition| {
        #[action("Choose the middle value as the pivot; set it aside at the end.")]
        let (mut range, mut lower, mut cursor, mut upper, pivot) = |partition| {
            let pivot = partition.len() - 1;
            partition.swap(partition.len() / 2, pivot);
            (partition, 0, 0, pivot, pivot)
        };

        #[cycle("Group the other values around the pivot.")]
        let classified = loop {
            #[question("Are any values unclassified?")]
            #[yes("YES")]
            #[no("NO")]
            let (select, classified) = |cursor, upper| cursor < upper;

            #[action("Select the first unclassified value.")]
            let value = |select, &range, cursor| &range[cursor];

            #[choice("How does this value compare with the pivot?")]
            #[case("Less than the pivot.")]
            #[case("Equal to the pivot.")]
            #[case("Greater than the pivot.")]
            let (less, equal, greater) = |value, &range, pivot| match value.cmp(&range[pivot]) {
                Ordering::Less => (),
                Ordering::Equal => (),
                Ordering::Greater => (),
            };

            #[action("Put this value in the left group.")]
            let stepped = |less, &mut range, &mut lower, &mut cursor| {
                range.swap(*cursor, *lower);
                *lower += 1;
                *cursor += 1;
            };

            #[action("Keep this value in the middle group.")]
            let stepped = |equal, &mut cursor| *cursor += 1;

            #[action("Swap this value into the right group; check its replacement next.")]
            let stepped = |greater, &mut range, cursor, &mut upper| {
                *upper -= 1;
                range.swap(cursor, *upper);
            };

            |stepped| continue;
        };

        #[action("Place the pivot with its equals; separate the left and right groups.")]
        let recur = |classified, range, lower, upper, pivot| {
            range.swap(upper, pivot);
            let (left, rest) = range.split_at_mut(lower);
            let (_, right) = rest.split_at_mut(upper + 1 - lower);
            if left.len() <= right.len() {
                (left, right)
            } else {
                (right, left)
            }
        };
    };

    #[stage("Sort the smaller group.")]
    let values = |recur| {
        #[action("Take the smaller and larger outer groups.")]
        let (smaller, larger) = |recur| recur;

        #[call("Sort the smaller group recursively.")]
        let sorted_part = |smaller| quick_sort(smaller);

        #[action("Continue sorting the larger group.")]
        let values = |sorted_part, larger| larger;
    };

    #[stage("Finish sorting.")]
    |finish| {
        return;
    };
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
