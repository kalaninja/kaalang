//! The traversal of `for_each`, written out with a loop cycle: a cursor and an
//! explicit choice between an item and exhaustion. This shape takes items one at
//! a time where a for cycle cannot, for instance to stop early or to advance
//! unevenly. A wider accumulator keeps sums of large i32 values representable.

use kaalang::kaa;

#[kaa]
pub(super) fn iterator_loop(values: &[i32]) -> i64 {
    #[kaa::action("🧮 Start with a total of zero.")]
    let mut total = || 0;

    #[kaa::action("👉 Start before the first value.")]
    let mut cursor = |values| values.iter();

    #[kaa::cycle("Add the list's values to the total one by one.")]
    let done = loop {
        #[kaa::choice("What comes next in the list?")]
        #[kaa::case("Another value.")]
        #[kaa::case("The end of the list.")]
        let (value, done) = |&mut cursor| match cursor.next() {
            Some(next) => next,
            None => (),
        };

        #[kaa::action("➕ Add this value to the running total.")]
        |value, &mut total| *total += i64::from(*value);

        |value| continue;
    };

    |done, total| return total;
}
