//! In-place three-way quicksort. Recursing on the smaller side and iterating
//! over the larger one keeps the call stack logarithmic.

use core::cmp::Ordering;

use kaalang::kaa;

#[kaa]
pub(crate) fn quick_sort<T: Ord>(values: &mut [T]) {
    #[kaa::stage("Check the range.")]
    let (partition, finish) = |values| {
        #[kaa::question("Are there at least two values?")]
        #[kaa::yes("YES")]
        #[kaa::no("NO")]
        let (split, finish) = |&values| values.len() > 1;

        #[kaa::action("Take the unsorted range.")]
        let partition = |split, values| values;
    };

    #[kaa::stage("Partition the range.")]
    let recur = |partition| {
        #[kaa::action("Choose the middle value as the pivot; set it aside at the end.")]
        let (mut range, mut lower, mut cursor, mut upper, pivot) = |partition| {
            let pivot = partition.len() - 1;
            partition.swap(partition.len() / 2, pivot);
            (partition, 0, 0, pivot, pivot)
        };

        #[kaa::cycle("Group the other values around the pivot.")]
        let classified = loop {
            #[kaa::question("Are any values unclassified?")]
            #[kaa::yes("YES")]
            #[kaa::no("NO")]
            let (select, classified) = |cursor, upper| cursor < upper;

            #[kaa::action("Select the first unclassified value.")]
            let value = |select, &range, cursor| &range[cursor];

            #[kaa::choice("How does this value compare with the pivot?")]
            #[kaa::case("Less than the pivot.")]
            #[kaa::case("Equal to the pivot.")]
            #[kaa::case("Greater than the pivot.")]
            let (less, equal, greater) = |value, &range, pivot| match value.cmp(&range[pivot]) {
                Ordering::Less => (),
                Ordering::Equal => (),
                Ordering::Greater => (),
            };

            #[kaa::action("Put this value in the left group.")]
            let stepped = |less, &mut range, &mut lower, &mut cursor| {
                range.swap(*cursor, *lower);
                *lower += 1;
                *cursor += 1;
            };

            #[kaa::action("Keep this value in the middle group.")]
            let stepped = |equal, &mut cursor| *cursor += 1;

            #[kaa::action("Swap this value into the right group; check its replacement next.")]
            let stepped = |greater, &mut range, cursor, &mut upper| {
                *upper -= 1;
                range.swap(cursor, *upper);
            };

            |stepped| continue;
        };

        #[kaa::action("Place the pivot with its equals; separate the left and right groups.")]
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

    #[kaa::stage("Sort the smaller group.")]
    let values = |recur| {
        #[kaa::action("Take the smaller and larger outer groups.")]
        let (smaller, larger) = |recur| recur;

        #[kaa::call("Sort the smaller group recursively.")]
        let sorted_part = |smaller| quick_sort(smaller);

        #[kaa::action("Continue sorting the larger group.")]
        let values = |sorted_part, larger| larger;
    };

    #[kaa::stage("Finish sorting.")]
    |finish| {
        return;
    };
}
