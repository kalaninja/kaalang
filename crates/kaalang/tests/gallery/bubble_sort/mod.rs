//! Bubble sort. Each pass walks the unsorted prefix once and swaps every pair
//! it finds out of order, so the largest value left reaches its place. The
//! prefix shrinks by one after every pass, and the flow ends when a single
//! value is left to place.

use kaalang::kaalang;

#[kaalang]
fn bubble_sort(values: &mut [i32]) {
    #[action("📏 Every value is still unplaced.")]
    let unsorted = |&values| values.len();

    #[cycle("🫧 Float the largest unsorted value to its place.")]
    let sorted = |mut values, mut unsorted| {
        #[question("Is more than one value still unplaced?")]
        #[yes("YES")]
        #[no("NO")]
        let (pass, done) = |unsorted| unsorted > 1;

        #[action("⏮️ Start at the second value.")]
        let index = |pass| 1;

        #[cycle("Compare every adjacent pair in the pass.")]
        let compared = |&mut values, unsorted, mut index| {
            #[question("Is there another pair in this pass?")]
            #[yes("YES")]
            #[no("NO")]
            let (compare, complete) = |index, unsorted| index < unsorted;

            |complete| break;

            #[question("Does the earlier value exceed the later one?")]
            #[yes("YES")]
            #[no("NO")]
            let (greater, stepped) = |compare, &values, index| values[index - 1] > values[index];

            #[action("🔀 Swap the pair.")]
            let stepped = |greater, &mut values, index| values.swap(index - 1, index);

            #[action("⏭️ Move on to the next pair.")]
            |stepped, &mut index| *index += 1;
        };

        #[action("🔻 One more value is in its place.")]
        |compared, &mut unsorted| *unsorted -= 1;

        |done| break;
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
