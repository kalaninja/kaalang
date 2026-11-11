//! Bubble sort with nested cycles. Each pass ends the unsorted prefix at its
//! last swap; a pass without swaps finishes the sort.

use kaalang::kaa;

#[kaa]
pub(super) fn bubble_sort(mut values: &mut [i32]) {
    #[kaa::action("📏 Start with the whole list to sort.")]
    let mut unsorted = |&values| values.len();

    #[kaa::cycle("🫧 Move larger values to the right.")]
    let sorted = loop {
        #[kaa::question("Are at least two values left to sort?")]
        #[kaa::yes("YES")]
        #[kaa::no("NO")]
        let (pass, sorted) = |unsorted| unsorted > 1;

        #[kaa::action("⏮️ Begin the pass with no swaps recorded.")]
        let mut last_swap = |pass| 0;

        #[kaa::cycle("Put each adjacent pair in order, from left to right.")]
        let compared = |pass, unsorted| {
            for index in 1..unsorted {
                #[kaa::action("Select the two values in this adjacent pair.")]
                let (left, right) = |index, &values| (values[index - 1], values[index]);

                #[kaa::question("Is the left value greater than the right?")]
                #[kaa::yes("YES")]
                #[kaa::no("NO")]
                let (greater, _ordered) = |left, right| left > right;

                #[kaa::action("🔀 Swap the values; remember where this swap happened.")]
                let _ordered = |greater, &mut values, index, &mut last_swap| {
                    values.swap(index - 1, index);
                    *last_swap = index;
                };
            }
        };

        #[kaa::question("Did this pass swap any values?")]
        #[kaa::yes("YES")]
        #[kaa::no("NO")]
        let (shrink, sorted) = |compared, last_swap| last_swap > 0;

        #[kaa::action("🔻 Leave the sorted tail after the last swap out of the next pass.")]
        |shrink, last_swap, &mut unsorted| *unsorted = last_swap;

        |shrink| continue;
    };

    |sorted| return sorted;
}
