//! Iterator traversal: a for cycle takes the list's values one by one.
//! `iterator_loop` writes the same traversal out with a loop cycle.
//! A wider accumulator keeps sums of large i32 values representable.

use kaalang::kaa;

#[kaa]
pub(super) fn for_each(values: &[i32]) -> i64 {
    #[kaa::action("🧮 Start with a total of zero.")]
    let mut total = || 0;

    #[kaa::cycle("Add the list's values to the total one by one.")]
    let done = |values| {
        for value in values {
            #[kaa::action("➕ Add this value to the running total.")]
            |value, &mut total| *total += i64::from(*value);
        }
    };

    |done, total| return total;
}
