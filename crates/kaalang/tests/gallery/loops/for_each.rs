//! Iterator traversal: a for cycle takes the list's values one by one.
//! `iterator_loop` writes the same traversal out with a loop cycle.
//! A wider accumulator keeps sums of large i32 values representable.

use kaalang::kaalang;

#[kaalang]
fn for_each(values: &[i32]) -> i64 {
    #[action("🧮 Start with a total of zero.")]
    let mut total = || 0;

    #[cycle("Add the list's values to the total one by one.")]
    let done = |values| {
        for value in values {
            #[action("➕ Add this value to the running total.")]
            |value, &mut total| *total += i64::from(*value);
        }
    };

    |done, total| return total;
}

#[test]
fn for_each_adds_up_every_value() {
    assert_eq!(for_each(&[]), 0);
    assert_eq!(for_each(&[7]), 7);
    assert_eq!(for_each(&[1, 2, 3, 4]), 10);
    assert_eq!(for_each(&[5, -5, 5]), 5);
    assert_eq!(for_each(&[i32::MAX, i32::MAX]), 2 * i64::from(i32::MAX));
    assert_eq!(for_each(&[i32::MIN, i32::MIN]), 2 * i64::from(i32::MIN));
}
