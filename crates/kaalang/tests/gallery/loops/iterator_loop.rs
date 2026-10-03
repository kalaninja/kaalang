//! The traversal of `for_each`, written out with a loop cycle: a cursor and an
//! explicit choice between an item and exhaustion. This shape takes items one at
//! a time where a for cycle cannot, for instance to stop early or to advance
//! unevenly. A wider accumulator keeps sums of large i32 values representable.

use kaalang::kaalang;

#[kaalang]
fn iterator_loop(values: &[i32]) -> i64 {
    #[action("🧮 Start with a total of zero.")]
    let mut total = || 0;

    #[action("👉 Start before the first value.")]
    let mut cursor = |values| values.iter();

    #[cycle("Add the list's values to the total one by one.")]
    let done = loop {
        #[choice("What comes next in the list?")]
        #[case("Another value.")]
        #[case("The end of the list.")]
        let (value, done) = |&mut cursor| match cursor.next() {
            Some(next) => next,
            None => (),
        };

        #[action("➕ Add this value to the running total.")]
        |value, &mut total| *total += i64::from(*value);

        |value| continue;
    };

    |done, total| return total;
}

#[test]
fn iterator_loop_adds_up_every_value() {
    assert_eq!(iterator_loop(&[]), 0);
    assert_eq!(iterator_loop(&[7]), 7);
    assert_eq!(iterator_loop(&[1, 2, 3, 4]), 10);
    assert_eq!(iterator_loop(&[5, -5, 5]), 5);
    assert_eq!(
        iterator_loop(&[i32::MAX, i32::MAX]),
        2 * i64::from(i32::MAX)
    );
    assert_eq!(
        iterator_loop(&[i32::MIN, i32::MIN]),
        2 * i64::from(i32::MIN)
    );
}
