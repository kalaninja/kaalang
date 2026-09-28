//! Bubble sort with nested cycles. Each pass ends the unsorted prefix at its
//! last swap; a pass without swaps finishes the sort.

use kaalang::kaalang;

#[kaalang]
fn bubble_sort(mut values: &mut [i32]) {
    #[action("📏 Start with the whole list to sort.")]
    let mut unsorted = |&values| values.len();

    #[cycle("🫧 Move larger values to the right.")]
    let sorted = {
        #[question("Are at least two values left to sort?")]
        #[yes("YES")]
        #[no("NO")]
        let (pass, sorted) = |unsorted| unsorted > 1;

        #[action("⏮️ Start at the first pair; no swaps have been made.")]
        let (mut index, mut last_swap) = |pass| (1, 0);

        #[cycle("Put each adjacent pair in order, from left to right.")]
        let compared = |index| {
            #[question("Is there another pair in this pass?")]
            #[yes("YES")]
            #[no("NO")]
            let (select, compared) = |index, unsorted| index < unsorted;

            #[action("Select the two values in the next adjacent pair.")]
            let (left, right) = |select, &values, index| (values[index - 1], values[index]);

            #[question("Is the left value greater than the right?")]
            #[yes("YES")]
            #[no("NO")]
            let (greater, stepped) = |left, right| left > right;

            #[action("🔀 Swap the values; remember where this swap happened.")]
            let stepped = |greater, &mut values, index, &mut last_swap| {
                values.swap(index - 1, index);
                *last_swap = index;
            };

            #[action("⏭️ Move on to the next pair.")]
            |stepped, &mut index| *index += 1;

            |stepped| continue;
        };

        #[question("Did this pass swap any values?")]
        #[yes("YES")]
        #[no("NO")]
        let (shrink, sorted) = |compared, last_swap| last_swap > 0;

        #[action("🔻 Leave the sorted tail after the last swap out of the next pass.")]
        |shrink, last_swap, &mut unsorted| *unsorted = last_swap;

        |shrink| continue;
    };

    |sorted| return sorted;
}

#[test]
fn bubble_sort_orders_the_values_in_place() {
    for values in [
        [].as_slice(),
        &[7],
        &[1, 2, 3, 4],
        &[4, 3, 2, 1],
        &[2, 1, 3, 4],
        &[1, 3, 2, 4],
        &[2, 3, 1, 4],
        &[3, -1, 3, 0, -1],
        &[5, 5, 5],
        &[-8, 12, -8, 0, 7, -3],
    ] {
        let mut expected = values.to_vec();
        expected.sort_unstable();

        let mut sorted = values.to_vec();
        bubble_sort(&mut sorted);

        assert_eq!(sorted, expected);
    }
}
