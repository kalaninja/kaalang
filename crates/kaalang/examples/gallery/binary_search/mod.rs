//! Binary search over a sorted slice, shown with two branch orders.

use std::cmp::Ordering;

use kaalang::kaa;

#[kaa]
fn binary_search(values: &[i32], target: i32) -> Option<usize> {
    #[kaa::action("📏 Start with the entire sorted list.")]
    let (mut left, mut right) = |values| (0, values.len());

    #[kaa::cycle("🔍 Narrow the range until the target is found or ruled out.")]
    let result = loop {
        #[kaa::question("Are any values left in the search range?")]
        #[kaa::yes("YES")]
        #[kaa::no("NO")]
        let (iterate, leave) = |left, right| left < right;

        #[kaa::action("🚫 Report that the target is absent.")]
        let result = |leave| None;

        #[kaa::action(r"📍 Select the middle value, $$\mathrm{mid} = \mathrm{left} + \left\lfloor \frac{\mathrm{right} - \mathrm{left}}{2} \right\rfloor$$.")]
        let (mid, value) = |iterate, values, left, right| {
            let mid = left + (right - left) / 2;
            (mid, values[mid])
        };

        #[kaa::choice("How does this value compare with the target?")]
        #[kaa::case("Less than the target.")]
        #[kaa::case("Greater than the target.")]
        #[kaa::case("Equal to the target.")]
        let (less, greater, equal) = |value, target| match value.cmp(&target) {
            Ordering::Less => (),
            Ordering::Greater => (),
            Ordering::Equal => (),
        };

        #[kaa::action("➡️ Discard this value and everything to its left.")]
        let stepped = |less, mid, &mut left| *left = mid + 1;

        #[kaa::action("⬅️ Discard this value and everything to its right.")]
        let stepped = |greater, mid, &mut right| *right = mid;

        #[kaa::action("🎯 Report the position of this matching value.")]
        let result = |equal, mid| Some(mid);

        |stepped| continue;
    };

    |result| return result;
}

#[kaa]
fn binary_search_swapped(values: &[i32], target: i32) -> Option<usize> {
    #[kaa::action("📏 Start with the entire sorted list.")]
    let (mut left, mut right) = |values| (0, values.len());

    #[kaa::cycle("🔍 Narrow the range until the target is found or ruled out.")]
    let result = loop {
        #[kaa::question("Are any values left in the search range?")]
        #[kaa::no("NO")]
        #[kaa::yes("YES")]
        let (leave, iterate) = |left, right| left < right;

        #[kaa::action("🚫 Report that the target is absent.")]
        let result = |leave| None;

        #[kaa::action(r"📍 Select the middle value, $$\mathrm{mid} = \mathrm{left} + \left\lfloor \frac{\mathrm{right} - \mathrm{left}}{2} \right\rfloor$$.")]
        let (mid, value) = |iterate, values, left, right| {
            let mid = left + (right - left) / 2;
            (mid, values[mid])
        };

        #[kaa::choice("How does this value compare with the target?")]
        #[kaa::case("Equal to the target.")]
        #[kaa::case("Less than the target.")]
        #[kaa::case("Greater than the target.")]
        let (equal, less, greater) = |value, target| match value.cmp(&target) {
            Ordering::Equal => (),
            Ordering::Less => (),
            Ordering::Greater => (),
        };

        #[kaa::action("🎯 Report the position of this matching value.")]
        let result = |equal, mid| Some(mid);

        #[kaa::action("➡️ Discard this value and everything to its left.")]
        let stepped = |less, mid, &mut left| *left = mid + 1;

        #[kaa::action("⬅️ Discard this value and everything to its right.")]
        let stepped = |greater, mid, &mut right| *right = mid;

        |stepped| continue;
    };

    |result| return result;
}

pub(super) fn check() {
    for values in [
        &[][..],
        &[5][..],
        &[-4, -1, 0, 3, 7][..],
        &[1, 1, 1, 3, 3][..],
    ] {
        for target in -6..=9 {
            for found in [
                binary_search(values, target),
                binary_search_swapped(values, target),
            ] {
                assert_eq!(found.is_some(), values.binary_search(&target).is_ok());
                if let Some(index) = found {
                    assert_eq!(values[index], target);
                }
            }
        }
    }
}
