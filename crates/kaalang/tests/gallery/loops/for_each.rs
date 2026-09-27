//! Iterator traversal with an explicit choice between an item and exhaustion.
//! A wider accumulator keeps sums of large i32 values representable.

use kaalang::kaalang;

#[kaalang]
fn for_each(values: &[i32]) -> i64 {
    #[action("🧮 Start the iterator and total.")]
    let (mut cursor, mut total) = |values| (values.iter(), 0);

    #[cycle("Add every value to the total.")]
    let done = {
        #[choice("Is there another value?")]
        #[case("Next value.")]
        #[case("No values left.")]
        let (value, done) = |&mut cursor| match cursor.next() {
            Some(next) => next,
            None => (),
        };

        #[action("➕ Add it to the total.")]
        |value, &mut total| *total += i64::from(*value);

        |value| continue;
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
